/*!
Thyme is a highly customizable, themable immediate mode GUI toolkit for Rust.

It is designed to be performant and flexible enough for use both in prototyping and production games and applications.
Requiring a theme and image sources adds some additional development cost compared to many other immediate mode toolkits,
however the advantage is full flexibility and control over the ultimate appearance of your UI.

To use Thyme, you need the core library, a renderer (one based on [Glium](https://github.com/glium/glium) is included),
event handling support (one based on [winit](https://github.com/rust-windowing/winit) is included), and a theme definition
with associated images and fonts.

All thyme widgets are drawn using images, with the image data registered with the renderer, and then individual
widget components defined within that image within the theme file.  Likewise, `ttf` fonts are registered with
the renderer and then individual fonts for use in your UI are defined in the theme file.
Widgets themselves can be defined fully in source code, with only some basic templates in the theme file, or
you can largely leave only logic in the source, with layout, alignment, etc defined in the theme file.

# Overview

In general, you first create the [`ContextBuilder`](struct.ContextBuilder.html) and register resources with it.
Once done, you [`build`](struct.ContextBuilder.html#method.build) the associated [`Context`](struct.Context.html).
At each frame of your app, you [`create a Thyme frame`](struct.Context.html#method.create_frame).  The
[`Frame`](struct.Frame.html) is then passed along through your UI building routines, and is used to create
[`WidgetBuilders`](struct.WidgetBuilder.html) and populate your Widget tree.

# Theme Definition
When creating a [`ContextBuilder`](struct.ContextBuilder.html), you need to specify a theme.  You can keep the
theme fairly small with just a base set of widgets, defining most things in code, or go the other way around.

The theme can be defined from any [`serde`](https://serde.rs/)
compatible source, with the examples in this project using [`YAML`](https://yaml.org/).
The theme has several sections: `fonts`, `image_sets`, and `widgets`.

## Fonts
Defining fonts is very simple.  The `fonts` section consists of a mapping, with `IDs` mapped
to font data.  The font IDs are used elsewhere in the widgets section and in code when specifying
a [`font`](struct.WidgetBuilder.html#method.font).

The data consists of a `source`, which is a string which must match one of the fonts registered
with the [`ContextBuilder`](struct.ContextBuilder.html#method.register_font_source), and a `size`
in logical pixels.
```yaml
fonts:
  medium:
    source: roboto
    size: 20
  small:
    source: roboto
    size: 16
```

## Image Sets
Images are defined as a series of `image_sets`.  Each image_set has an `id`, used as the first
part of the ID of each image in the set.  The complete image ID is equal to `image_set_id/image_id`.
Each image_set may be `source`d from a different image file.
Each image file must be registered with [`ContextBuilder`](struct.ContextBuilder.html#method.register_image),
under an ID matching the `source` id.
```yaml
image_sets:
  source: gui
  images:
    ...
```

### Images
Each image set can contain many `images`, which are defined as subsets of the overall image file in various ways.  The type of
image for each image within the set is determined based on the parameters specified.

#### Simple Images
Simple images are defined by a position and size, in pixels, within the overall image.  The `fill` field is optional, with valid
values of `None` (default) - image is drawn at fixed size, `Stretch` - image is stretched to fill an area, `Repeat` - image repeats
over an area.
```yaml
  progress_bar:
    position: [100, 100]
    size: [16, 16]
    fill: Stretch
```

#### Composed Images
Composed images consist of a 3 by 3 grid.  The corners are drawn at a fixed size, while the middle sections stretch along
one axis.  The center grid image stretches to fill in the inner area of the image.  These images allow you to easily draw
widgets with almost any size that maintain the same look.  The `grid_size` specifies the size of one of the 9 cells, with
each cell having the same size.
```yaml
  button_normal:
    position: [100, 100]
    grid_size: [16, 16]
```

#### Composed Horizontal and Vertical
There are also composed horizontal and composed vertical images, that consist of a 3x1 and 1x3 grid, respectively.  These
are defined and used in the same manner as regular composed images, but use `grid_size_horiz` and `grid_size_vert` to
differentiate the different types.

#### Timed Images
Timed images display one out of several frames, on a timer.  Timed images can repeat continuously (the default), or only display once,
based on the value of the optional `once` parameter.  `frame_time_millis` is how long each frame is shown for, in milliseconds.  Each
`frame` is the `id` of an image within the current image set.  It can be a Simple Image or Composed Image.
```yaml
  button_flash:
    frame_time_millis: 500
    once: false
    frames:
      - button_normal
      - button_bright
```

#### Animated Images
Animated images display one of several sub images based on the [`AnimState`](struct.AnimState.html). of the parent widget.
The referenced images are specified by `id`, and can include Simple, Composed, and Timed images.
```yaml
  button:
    states:
      Normal: button_normal
      Hover: button_hover
      Pressed: button_pressed
      Active: button_active
      Active + Hover: button_hover_active
      Active + Pressed: button_pressed_active
```

## Widgets
The widgets section defines themes for all widgets you will use in your UI.  Whenever you create a widget, such as through
[`Frame.start`](struct.Frame.html#method.start), you specify a `theme_id`.  This `theme_id` must match one
of the keys defined in this section.

### Recursive definition
Widget themes are defined recursively, and Thyme will first look for the exact recursive match, before falling back to the top level match.
Each widget entry may have one or more `children`, with each child being a full widget definition in its own right.  The ID of each widget in the
tree is computed as `{parent_id}/{child_id}`, recursively.

For example, if you specified a `button` that is a child of a `content` that is in turn a child of `window`, the theme ID will be `window/content/button`.
Thyme will first look for a theme at the full ID, i.e.
```yaml
  window:
    children:
      content:
        children:
          button
```
If that is not found, it will look for `button` at the top level.  The [`child_align`](struct.WidgetBuilder.html#method.child_align),
[`layout`](struct.WidgetBuilder.html#method.layout), and [`layout_spacing`](struct.WidgetBuilder.html#method.layout_spacing) fields deal specifically with how
the widget will layout its children.

### Widget `from` attribute
Each widget entry in the `widgets` section may have a `from` attribute, which instructs Thyme to copy the specified widget theme into this theme.
This is resolved fully recursively and will copy all children, merging  where appropriate.  `from` attributes may also be defined recursively.
Specifically defined attributes within a widget theme will override the `from` theme.

For example, this definition:
```yaml
  button:
    background: gui/button
    size: [100, 25]
  titlebar:
    from: button
    children:
      label:
        font: medium
      close_button:
        from: button
        foreground: gui/close
        size: [25, 25]
  main_window_titlebar:
    from: titlebar
    children:
      label:
        text: "Main Window"
```

will interpret `main_window_titlebar` into the equivalent of this:
```yaml
  main_window_titlebar:
    background: gui/button
    size: [100, 25]
    children:
      label:
        font: medium
        text: "Main Window"
      close_button:
        background: gui/button
        foregorund: gui/close
        size: [25, 25]
```

### Widget Attributes
Each widget theme has many optional attributes that may be defined in the theme file, UI building source code, or both.  Source code
methods on [`WidgetBuilder`](struct.WidgetBuilder.html) will take precedence over items defined in the theme file.

```yaml
   complicated_button:
     text: Hello
     text_color: "#FFAA00"
     text_align: Center
     font: medium
     background: gui/button
     foreground: gui/button_icon
     wants_mouse: true
     wants_scroll: false
     pos: [10, 10]
     size: [100, 0]
     width_from: Normal
     height_from: FontLine
     border: { all: 5 }
     align: TopLeft
     child_align: Top
     layout: Vertical
     layout_spacing: 5
!*/

pub mod bench;
pub mod log;

mod context;
mod font;
mod frame;
mod glium_backend;
mod image;
mod theme;
mod recipes;
mod render;
mod theme_definition;
mod point;
mod scrollpane;
mod widget;
mod window;
mod winit_io;

pub use frame::Frame;
pub use point::{Rect, Point, Border};
pub use widget::{WidgetBuilder, WidgetState};
pub use context::{Context, ContextBuilder, PersistentState};
pub use scrollpane::{ScrollpaneBuilder, ShowElement};
pub use theme_definition::{AnimStateKey, AnimState, Align, Color, Layout, WidthRelative, HeightRelative};
pub use window::WindowBuilder;
pub use winit_io::WinitIo;
pub use glium_backend::GliumRenderer;
pub use render::{IO, Renderer};

/// A generic error that can come from a variety of internal sources.
#[derive(Debug, Clone)]
pub enum Error {
    Theme(String),
    FontSource(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use self::Error::*;
        match self {
            Theme(msg) => write!(f, "Error creating theme from theme definition: {}", msg),
            FontSource(msg) => write!(f, "Error reading font source: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use self::Error::*;
        match self {
            Theme(..) => None,
            FontSource(..) => None,
        }
    }
}