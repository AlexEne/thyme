use std::collections::{HashMap};

use crate::theme_definition::{
    ThemeDefinition, ImageDefinition, ImageDefinitionKind, WidgetThemeDefinition,
};
use crate::font::{Font, FontSummary, FontHandle, FontSource};
use crate::image::{Image, ImageHandle};
use crate::render::{TextureData, Renderer};
use crate::{Color, Error, Point, Border, Align, Layout, WidthRelative, HeightRelative};

pub struct ThemeSet {
    fonts: Vec<Font>,
    font_handles: HashMap<String, FontSummary>,

    images: Vec<Image>,
    image_handles: HashMap<String, ImageHandle>,

    theme_handles: HashMap<String, WidgetThemeHandle>,
    themes: Vec<WidgetTheme>,
}

impl ThemeSet {
    pub(crate) fn new<R: Renderer>(
        definition: ThemeDefinition,
        textures: HashMap<String, TextureData>,
        font_sources: HashMap<String, FontSource>,
        renderer: &mut R,
    ) -> Result<ThemeSet, Error> {
        let mut font_handles = HashMap::new();
        let mut font_handle = FontHandle::default();
        let mut fonts = Vec::new();
        for (font_id, font) in definition.fonts {
            let source = font_sources.get(&font.source).ok_or_else(||
                Error::Theme(format!("Unable to locate font handle {}", font.source))
            )?;

            let font = renderer.register_font(font_handle, source, font.size)?;
            font_handle = font_handle.next();

            let line_height = font.line_height();
            let handle = font.handle();
            assert!(handle.id() == fonts.len());
            fonts.push(font);
            font_handles.insert(font_id, FontSummary { handle, line_height });
        }

        let mut images = HashMap::new();
        for (set_id, set) in definition.image_sets {
            let mut images_in_set = HashMap::new();

            let texture = textures.get(&set.source).ok_or_else(||
                Error::Theme(format!("Unable to locate texture {}", set.source))
            )?;

            let mut animated_images:Vec<(String, ImageDefinition)> = Vec::new();

            // first parse all images without dependencies
            for (image_id, image_def) in set.images {
                match image_def.kind {
                    ImageDefinitionKind::Animated { .. } => animated_images.push((image_id, image_def)),
                    _ => {
                        let image = Image::new(&image_id, image_def, texture, &images_in_set)?;
                        images_in_set.insert(image_id, image);
                    }
                }
            }

            // now parse animated images
            for (id, image_def) in animated_images {
                let image = Image::new(&id, image_def, texture, &images_in_set)?;
                images_in_set.insert(id, image);
            }

            for (id, image) in images_in_set {
                images.insert(format!("{}/{}", set_id, id), image);
            }
        }

        let mut images_out = Vec::new();
        let mut image_handles = HashMap::new();
        for (index, (id, image)) in images.into_iter().enumerate() {
            let handle = ImageHandle { id: index };
            images_out.push(image);
            image_handles.insert(id, handle);
        }

        // build the set of themes
        let mut theme_handles = HashMap::new();
        let mut themes = Vec::new();
        let mut handle_index = 0;
        for (theme_id, theme) in definition.widgets {
            WidgetTheme::create(
                "",
                None,
                theme_id, 
                &mut handle_index, 
                &mut theme_handles, 
                &mut themes, 
                theme, 
                &image_handles,
                &font_handles,
            )?;
        }

        // recursively resolve all "from" theme references

        // we may need to loop several times in order to resolve nested references
        const MAX_ITERATIONS: i32 = 20;
        let mut iteration = 0;
        loop {
            if iteration == MAX_ITERATIONS {
                return Err(
                    Error::Theme(format!("Unable to resolve all from references after {} iterations.  \
                        This is most likely caused by a circular reference.", iteration))
                );
            }

            let to_ids: Vec<WidgetThemeHandle> = theme_handles.values().copied().collect();
            let mut found_new = false;

            for to_id in to_ids.iter() {
                let from_str = match &themes[to_id.id as usize].from {
                    None => continue,
                    Some(from_id) => from_id,
                };

                found_new = true;

                let from_id = *theme_handles.get(from_str).ok_or_else(|| {
                    Error::Theme(format!("Invalid from theme '{}' in '{}'", from_str, themes[to_id.id as usize].id))
                })?;

                // if the 'from' field has its own 'from', don't resolve
                // it yet.  we need the nested froms to resolve first
                // in order to populate all fields correctly
                if themes[from_id.id as usize].from.is_some() { continue; }

                // we are definitely going to resolve the from, so now remove it
                themes[to_id.id as usize].from.take();

                merge_from(
                    from_id,
                    *to_id,
                    &mut themes,
                    &mut handle_index,
                    &mut theme_handles,
                )
            }

            if !found_new { break; }
            iteration += 1;
        }

        Ok(ThemeSet {
            font_handles,
            fonts,
            image_handles,
            images: images_out,
            theme_handles,
            themes,
        })
    }

    pub fn theme(&self, id: &str) -> Option<&WidgetTheme> {
        self.handle(id).map(|handle| &self.themes[handle.id as usize])
    }

    pub fn font(&self, handle: FontHandle) -> &Font {
        &self.fonts[handle.id()]
    }

    pub fn find_font(&self, id: Option<&str>) -> Option<FontSummary> {
        match id {
            None => None,
            Some(id) => {
                match self.font_handles.get(id) {
                    None => {
                        // TODO warn earlier and only once instead of on every frame
                        log::warn!("Invalid font when drawing: '{}'", id);
                        None
                    }, Some(font_sum) => {
                        Some(*font_sum)
                    }
                }
            }
        }
    }

    pub fn image(&self, handle: ImageHandle) -> &Image {
        &self.images[handle.id]
    }

    pub fn find_image(&self, id: Option<&str>) -> Option<ImageHandle> {
        match id {
            None => None,
            Some(id) => {
                match self.image_handles.get(id) {
                    None => {
                        // TODO warn earlier and only once instead of every frame like this will
                        log::warn!("Invalid image when drawing: '{}'", id);
                        None
                    }, Some(image) => Some(*image),
                }
            }
        }
    }

    pub fn handle(&self, id: &str) -> Option<WidgetThemeHandle> {
        self.theme_handles.get(id).cloned()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct WidgetThemeHandle {
    id: u64,
}

#[derive(Clone)]
pub struct WidgetTheme {
    from: Option<String>,
    pub full_id: String,

    pub id: String,
    pub parent_handle: Option<WidgetThemeHandle>,
    pub handle: WidgetThemeHandle,

    pub text: Option<String>,
    pub text_color: Option<Color>,
    pub font: Option<FontSummary>,
    pub background: Option<ImageHandle>,
    pub foreground: Option<ImageHandle>,

    // all fields are options instead of using default so
    // we can detect when to override them
    pub wants_mouse: Option<bool>,
    pub text_align: Option<Align>,
    pub pos: Option<Point>,
    pub size: Option<Point>,
    pub width_from: Option<WidthRelative>,
    pub height_from: Option<HeightRelative>,
    pub border: Option<Border>,
    pub align: Option<Align>,
    pub child_align: Option<Align>,
    pub layout: Option<Layout>,
    pub layout_spacing: Option<Point>,
    pub children: Vec<WidgetThemeHandle>,
}

impl WidgetTheme {
    #[allow(clippy::too_many_arguments)]
    fn create(
        parent_id: &str,
        parent_handle: Option<WidgetThemeHandle>,
        id: String,
        handle_index: &mut u64,
        handles: &mut HashMap<String, WidgetThemeHandle>,
        themes: &mut Vec<WidgetTheme>,
        def: WidgetThemeDefinition,
        images: &HashMap<String, ImageHandle>,
        fonts: &HashMap<String, FontSummary>,
    ) -> Result<WidgetThemeHandle, Error> {
        if id.contains('/') {
            return Err(
                Error::Theme(format!("'{}' theme name invalid.  the '/' character is not allowed", id))
            );
        }

        // handle top level as a special case
        let parent_id = if parent_id.is_empty() {
            id.to_string()
        } else {
            format!("{}/{}", parent_id, id)
        };

        let background = if let Some(bg) = def.background {
            Some(*images.get(&bg).ok_or_else(||
                Error::Theme(format!("Unable to locate image '{}' as background for widget '{}'", bg, parent_id))
            )?)
        } else {
            None
        };

        let foreground = if let Some(fg) = def.foreground {
            Some(*images.get(&fg).ok_or_else(||
                Error::Theme(format!("Unable to locate image '{}' as foreground for widget '{}'", fg, parent_id))
            )?)
        } else {
            None
        };

        let font = if let Some(font) = def.font {
            let font_handle = fonts.get(&font).ok_or_else(||
                Error::Theme(format!("Unable to locate font '{}' for widget '{}'", font, parent_id))
            )?;
            Some(*font_handle)
        } else {
            None
        };

        let handle = WidgetThemeHandle { id: *handle_index };
        *handle_index += 1;
        let theme = WidgetTheme {
            from: def.from,
            parent_handle,
            handle,
            id,
            full_id: parent_id.to_string(),
            text: def.text,
            text_color: def.text_color,
            font,
            background,
            foreground,
            wants_mouse: def.wants_mouse,
            text_align: def.text_align,
            pos: def.pos,
            size: def.size,
            width_from: def.width_from,
            height_from: def.height_from,
            align: def.align,
            child_align: def.child_align,
            border: def.border,
            layout: def.layout,
            layout_spacing: def.layout_spacing,
            children: Vec::new(),
        };

        themes.push(theme);

        let mut children = Vec::new();
        for (child_id, child_def) in def.children {
            let child = WidgetTheme::create(
                &parent_id,
                Some(handle),
                child_id,
                handle_index,
                handles,
                themes,
                child_def,
                images,
                fonts
            )?;
            children.push(child);
        }

        themes[handle.id as usize].children = children;

        handles.insert(parent_id, handle);

        Ok(handle)
    }
}

fn merge_from(
    from_id: WidgetThemeHandle,
    to_id: WidgetThemeHandle,
    themes: &mut Vec<WidgetTheme>,
    handle_index: &mut u64,
    theme_handles: &mut HashMap<String, WidgetThemeHandle>,
) {
    let from = themes[from_id.id as usize].clone();
    let from_children = from.children.clone();

    let to = &mut themes[to_id.id as usize];
    let to_children = to.children.clone();

    // preserve any as-yet unresolve child from refs
    to.from = from.from;

    if to.wants_mouse.is_none() { to.wants_mouse = from.wants_mouse; }
    if to.font.is_none() { to.font = from.font; }
    if to.background.is_none() { to.background = from.background; }
    if to.foreground.is_none() { to.foreground = from.foreground; }
    if to.text_align.is_none() { to.text_align = from.text_align; }
    if to.pos.is_none() { to.pos = from.pos; }
    if to.size.is_none() { to.size = from.size; }
    if to.width_from.is_none() { to.width_from = from.width_from; }
    if to.height_from.is_none() { to.height_from = from.height_from; }
    if to.border.is_none() { to.border = from.border; }
    if to.align.is_none() { to.align = from.align; }
    if to.child_align.is_none() { to.child_align = from.child_align; }
    if to.layout.is_none() { to.layout = from.layout; }
    if to.layout_spacing.is_none() { to.layout_spacing = from.layout_spacing; }

    for child_id in to_children.iter() {
        let mut merge = None;

        {
            let child = &themes[child_id.id as usize];
            
            for from_child_id in from_children.iter() {
                let from_child = &themes[from_child_id.id as usize];
                if from_child.id == child.id {
                    merge = Some(from_child_id);
                    break;
                }
            }
        }

        if let Some(from_id) = merge {
            merge_from(
                *from_id,
                *child_id,
                themes,
                handle_index,
                theme_handles,
            )
        }
    }

    for from_child_id in from_children.iter() {
        let mut found = false;

        {
            let from_child = &themes[from_child_id.id as usize];

            for to_child_id in to_children.iter() {
                let child = &themes[to_child_id.id as usize];
                if from_child.id == child.id {
                    found = true;
                    break;
                }
            }
        }

        if !found {
            add_children_recursive(
                *from_child_id,
                to_id,
                themes,
                handle_index,
                theme_handles,
            );
        }
    }
}

fn add_children_recursive(
    from_id: WidgetThemeHandle,
    to_id: WidgetThemeHandle,
    themes: &mut Vec<WidgetTheme>,
    handle_index: &mut u64,
    theme_handles: &mut HashMap<String, WidgetThemeHandle>,
) {
    let mut from = themes[from_id.id as usize].clone();

    let to = &mut themes[to_id.id as usize];
    let handle = WidgetThemeHandle { id: *handle_index };
    *handle_index += 1;

    let full_id = format!("{}/{}", to.full_id, from.id);

    from.full_id = full_id.to_string();
    from.handle = handle;
    from.parent_handle = Some(to_id);

    // take all the children out of our new theme and add them recursively
    // as new themes, rather than just making a shallow copy
    let from_children: Vec<_> = from.children.drain(..).collect();

    to.children.push(handle);
    themes.push(from);
    theme_handles.insert(full_id.clone(), handle);

    for from_child in from_children {
        {
            let from = &mut themes[from_child.id as usize];
            from.full_id = format!("{}/{}", full_id, from.id);
        }
        add_children_recursive(from_child, handle, themes, handle_index, theme_handles);
    }
}