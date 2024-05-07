use std::default::Default;

use poise::{CreateReply, serenity_prelude as serenity};
use serenity::all::{ButtonKind, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, Embed};
use serenity::all::{CreateActionRow, CreateButton, EditMessage, MessageId};
use serenity::builder::CreateMessage;

#[derive(Clone, Default, Debug, PartialEq, serde_derive::Deserialize, serde_derive::Serialize)]
pub struct Message {
    #[serde(default)]
    pub flags: Option<MessageFlags>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub components: Vec<Vec<serenity::all::Button>>,
    #[serde(default)]
    pub replace_embeds: Vec<Embed>,
    #[serde(default)]
    pub add_embeds: Vec<Embed>,
}
#[derive(Copy, Clone, Default, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde_derive::Deserialize, serde_derive::Serialize)]
pub enum MessageFlags {
    #[default]
    Reply,
    Ephemral,
    NoReply,
    Edit{id: MessageId}
}
macro_rules! recreate {
    ($builder:ident, $attr:ident, $($field: ident),+) => {
        $(
        if let Some($field) = $attr.$field {$builder = $builder.$field($field);}
        )+
    };
}
fn recreate_button_builder(button: serenity::all::Button) -> CreateButton{
    let mut builder = match button.data{
        ButtonKind::Link { url } => CreateButton::new_link(url),
        ButtonKind::NonLink { custom_id, style } => CreateButton::new(custom_id).style(style)
    };
    recreate!(builder, button, label, emoji);
    builder.disabled(button.disabled)
}
fn recreate_embed_builder(button: serenity::all::Embed) -> CreateEmbed {
    let mut builder = CreateEmbed::new();
    recreate!(builder, button, title, description, url, timestamp, colour);
    if let Some(image) = button.image {
        builder = builder.image(image.url);
    }
    if let Some(thumbnail) = button.thumbnail {
        builder = builder.thumbnail(thumbnail.url);
    }
    if let Some(footer) = button.footer {
        builder = builder.footer(recreate_embed_footer_builder(footer));
    }
    if let Some(author) = button.author {
        builder = builder.author(recreate_embed_author_builder(author));
    }
    builder = builder.fields(button.fields.into_iter().map(|field|(field.name, field.value, field.inline)));
    builder
}
fn recreate_embed_footer_builder(button: serenity::all::EmbedFooter) -> CreateEmbedFooter {
    let mut builder = CreateEmbedFooter::new(button.text);
    recreate!(builder, button, icon_url);
    builder
}
fn recreate_embed_author_builder(button: serenity::all::EmbedAuthor) -> CreateEmbedAuthor {
    let mut builder = CreateEmbedAuthor::new(button.name);
    recreate!(builder, button, url, icon_url);
    builder
}

fn to_create_action_row(components: Vec<Vec<serenity::all::Button>>) -> Vec<CreateActionRow> {
    components.into_iter().map(|row|{
        CreateActionRow::Buttons(row.into_iter().map(recreate_button_builder).collect())
    }).collect::<Vec<_>>()
}

impl Into<CreateReply> for Message {
    fn into(self) -> CreateReply {
        let mut reply = poise::CreateReply::default().reply(false);
        reply.content = self.content;
        if let Some(MessageFlags::Ephemral) = self.flags { reply.ephemeral = Some(true);}
        else {reply.ephemeral = Some(false)}
        if !self.components.is_empty(){
            reply = reply.components(to_create_action_row(self.components));
        }
        reply.embeds = Iterator::chain(
            self.replace_embeds.into_iter().map(recreate_embed_builder),
            self.add_embeds.into_iter().map(recreate_embed_builder)
        ).collect();
        reply
    }
}
impl Into<CreateMessage> for Message {
    fn into(self) -> CreateMessage {
        let mut message = serenity::CreateMessage::default();
        if let Some(content) = self.content { message = message.content(content) };
        if !self.components.is_empty(){
            message = message.components(to_create_action_row(self.components));
        }
        if !self.replace_embeds.is_empty() {
            message = message.embeds(self.replace_embeds.into_iter().map(recreate_embed_builder).collect());
        }
        if !self.add_embeds.is_empty() {
            message = message.add_embeds(self.add_embeds.into_iter().map(recreate_embed_builder).collect());
        }
        message
    }
}
impl Into<EditMessage> for Message {
    fn into(self) -> EditMessage {
        let mut edit = serenity::EditMessage::default();
        if let Some(content) = self.content { edit = edit.content(content); }
        if !self.components.is_empty(){
            edit = edit.components(to_create_action_row(self.components));
        }
        if !self.replace_embeds.is_empty() {
            edit = edit.embeds(self.replace_embeds.into_iter().map(recreate_embed_builder).collect());
        }
        if !self.add_embeds.is_empty() {
            edit = edit.add_embeds(self.add_embeds.into_iter().map(recreate_embed_builder).collect());
        }
        edit
    }
}