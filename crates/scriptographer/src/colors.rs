//! The colour constructors Scriptographer's scripts made colours with in Illustrator:
//! `new RGBColor(r, g, b)`, `new GrayColor(g)` and `new CMYKColor(c, m, y, k)`, their components
//! from 0 to 1, which make Paper.js's `Color`s.
//!
//! Paper.js's `Color` has no CMYK components, so a `CMYKColor` becomes the RGB Illustrator showed
//! for it. A script gets a `Color`: `color.type` is `"rgb"` or `"gray"`, and `instanceof RGBColor`
//! is false. `NoColor` and `SpotColor` are missing.

use crate::{Call, Engine, Error, Value, global_class};

/// Adds `RGBColor`, `GrayColor` and `CMYKColor` to the realm's globals.
pub(crate) fn install<E: Engine>(engine: &mut E) -> Result<(), Error> {
    let global = engine.global()?;
    let names = [
        ("RGBColor", engine.constructor("RGBColor", rgb_color)?),
        ("GrayColor", engine.constructor("GrayColor", gray_color)?),
        ("CMYKColor", engine.constructor("CMYKColor", cmyk_color)?),
    ];
    engine.with_call(|call| {
        for (name, function) in names {
            call.set(&global, name, &Value::Js(function))?;
        }
        Ok(())
    })
}

/// `new RGBColor(red, green, blue[, alpha])`, as `new Color(...)` takes them.
fn rgb_color<H: Clone>(call: &mut dyn Call<H>, _this: &Value<H>, args: &[Value<H>]) -> Result<Value<H>, Error> {
    make_color(call, args)
}

/// `new GrayColor(gray[, alpha])`.
fn gray_color<H: Clone>(call: &mut dyn Call<H>, _this: &Value<H>, args: &[Value<H>]) -> Result<Value<H>, Error> {
    make_color(call, args)
}

/// `new CMYKColor(cyan, magenta, yellow, black)`, as the RGB `Color` Illustrator showed.
fn cmyk_color<H: Clone>(call: &mut dyn Call<H>, _this: &Value<H>, args: &[Value<H>]) -> Result<Value<H>, Error> {
    let black = 1.0 - component(call, args, 3)?;
    let mut rgb = [Value::Number(0.0), Value::Number(0.0), Value::Number(0.0)];
    for (index, value) in rgb.iter_mut().enumerate() {
        *value = Value::Number((1.0 - component(call, args, index)?) * black);
    }
    make_color(call, &rgb)
}

/// The component at `index`, from 0 to 1; one that isn't there is 0, as Illustrator's were.
fn component<H>(call: &mut dyn Call<H>, args: &[Value<H>], index: usize) -> Result<f64, Error> {
    match args.get(index) {
        Some(value) => Ok(call.to_number(value)?.clamp(0.0, 1.0)),
        None => Ok(0.0),
    }
}

/// `new Color(...args)`.
fn make_color<H: Clone>(call: &mut dyn Call<H>, args: &[Value<H>]) -> Result<Value<H>, Error> {
    let color = global_class(call, "Color")?;
    call.construct(&color, args)
}
