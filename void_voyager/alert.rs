use rustrict::{Censor, Type};
use tracing::warn;
use void_codex::Sector;

/// Sends a Discord message about the input level.
///
/// If set in the config, attempt to send a Discord message using
/// every configured webhook URL with information about the level.
///
/// This is used to optionally notify one or more Discord channels
/// when a level is uploaded to Voyager.
pub fn send_discord_message(webhook_urls: Vec<String>, sector: &Sector) {
    if webhook_urls.is_empty() {
        return;
    }
    let emojis = format!("```{}```", sector.to_emoji_map());

    let name = sector.name();
    let mut censor = Censor::from_str(&name);
    let censor = censor
        .with_censor_threshold(Type::SEVERE | Type::SEVERE & Type::EVASIVE)
        .with_censor_first_character_threshold(Type::SEVERE | Type::SEVERE & Type::EVASIVE);

    let name = censor.censor();
    let description = sector.description();
    censor.reset(description.chars());
    let description = censor.censor();
    let author = sector.author();
    censor.reset(author.chars());
    let author = censor.censor();

    let mut image = sector.brand_image_bytes();
    let name = name.replace('*', "\\*");
    let description = description.replace('*', "\\*");
    let author = format!("by {author}").replace('*', "\\*");

    let mut embed = serde_json::json!({
        "embeds": [{
            "title": name,
            "thumbnail": {
                "url": "attachment://brand.png"
            },
            "fields": [{
                "name": author,
                "value": description,
            },
            {
                "name": "If I had to put it in terms of emoji, it would look like this:",
                "value": emojis
            }]
        }]
    })
    .to_string()
    .as_bytes()
    .to_vec();
    let mut form = br#"----boundary
Content-Disposition: form-data; name="payload_json"

"#
    .to_vec();
    form.append(&mut embed);
    let mut image_metadata = br#"
----boundary
Content-Disposition: form-data; name="file1"; filename="brand.png"
Content-Type: image/png

"#
    .to_vec();
    form.append(&mut image_metadata);
    form.append(&mut image);
    let mut terminator = b"
----boundary--"
        .to_vec();
    form.append(&mut terminator);
    for url in webhook_urls {
        let form = form.clone();
        tokio::task::spawn_blocking(move || {
            let message = ureq::post(url)
                .header("Content-Type", "multipart/form-data; boundary=--boundary")
                .send(&form);
            if let Err(why) = message {
                warn!("could not send discord webhook message on level upload! {why}");
            }
        });
    }
}
