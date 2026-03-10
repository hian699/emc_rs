use serenity::all::{Color, CreateEmbed};

fn base_embed(
    prefix: &str,
    title: &str,
    description: impl Into<String>,
    color: Color,
) -> CreateEmbed {
    CreateEmbed::new()
        .title(format!("{prefix} {title}"))
        .description(description)
        .color(color)
}

pub fn success_embed(title: &str, description: impl Into<String>) -> CreateEmbed {
    base_embed("[OK]", title, description, Color::from_rgb(46, 204, 113))
}

pub fn info_embed(title: &str, description: impl Into<String>) -> CreateEmbed {
    base_embed("[INFO]", title, description, Color::from_rgb(52, 152, 219))
}

pub fn warning_embed(title: &str, description: impl Into<String>) -> CreateEmbed {
    base_embed("[WARN]", title, description, Color::from_rgb(241, 196, 15))
}

pub fn error_embed(title: &str, description: impl Into<String>) -> CreateEmbed {
    base_embed("[ERROR]", title, description, Color::from_rgb(231, 76, 60))
}
