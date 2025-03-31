use crate::{Context, Error};
use poise::serenity_prelude::GuildId;
use poise::serenity_prelude::Permissions;
use poise::serenity_prelude::PermissionOverwrite;
use poise::serenity_prelude::{self as serenity, CreateChannel, CreateEmbed, CreateMessage};
use poise::serenity_prelude::PermissionOverwriteType;
use poise::serenity_prelude::Mentionable;

use crate::utils::*;

#[poise::command(
    slash_command,
    prefix_command,
    guild_only
)]
pub async fn ticket(
    ctx: Context<'_>,
    #[description = "Describe your issue"] issue: Option<String>,
) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.say("This command must be used in a guild").await?;
            return Ok(());
        }
    };
    let category_id = match get_ticket_category(guild_id.into()) {
        Some(id) => id,
        None => {
            ctx.say("Ticket system not configured. Admins must run `/setticket` first.").await?;
            return Ok(());
        }
    };
    let author = ctx.author();
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M");
    let channel_name = format!("ticket-{}-{}", author.name, timestamp);
    let issue_description = issue.unwrap_or_else(|| "No description provided".to_string());
    let mut permissions = vec![
        PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(author.id),
        },
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
            kind: PermissionOverwriteType::Role(GuildId::everyone_role(&guild_id)),
        }
    ];
    for role_id in get_ticket_roles(guild_id.into()) {
        permissions.push(PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::MANAGE_MESSAGES,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Role(serenity::RoleId::new(role_id)),
        });
    }
    let channel_builder = CreateChannel::new(channel_name)
        .kind(serenity::ChannelType::Text)
        .category(category_id)
        .topic(issue_description.clone())
        .permissions(permissions);
    let http = ctx.http();
    let channel = match guild_id.create_channel(&http, channel_builder).await {
        Ok(c) => c,
        Err(e) => {
            ctx.say(format!("Failed to create ticket channel: {}", e)).await?;
            return Ok(());
        }
    };
    if let Err(e) = channel.say(&http, format!("{} created this ticket", author.mention())).await {
        ctx.say(format!("Failed to send ticket message: {}", e)).await?;
        return Ok(());
    }
    let embed = CreateEmbed::new()
        .title("New Ticket")
        .description(issue_description)
        .color(serenity::Colour::DARK_GREEN);
    if let Err(e) = channel.send_message(&http, CreateMessage::new().embed(embed)).await {
        ctx.say(format!("Failed to send ticket embed: {}", e)).await?;
        return Ok(());
    }
    ctx.say(format!("Created your ticket: {}", channel.mention())).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    category = "Moderation",
    guild_only
)]
pub async fn closeticket(
    ctx: Context<'_>,
    #[description = "Reason for closing"] reason: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let channel = ctx.channel_id();
    let channel_info = channel.to_channel(&ctx.http()).await?;
    let channel_name = match &channel_info {
        serenity::Channel::Guild(guild_channel) => &guild_channel.name,
        _ => {
            ctx.say("This command can only be used in ticket channels").await?;
            return Ok(());
        }
    };
    if !channel_name.starts_with("ticket-") {
        ctx.say("This command can only be used in ticket channels").await?;
        return Ok(());
    }
    let reason = reason.unwrap_or_else(|| "No reason provided".to_string());
    let embed = CreateEmbed::new()
        .title("Ticket Closed")
        .description(&reason)
        .color(serenity::Colour::DARK_RED);
    channel.send_message(&ctx.http(), CreateMessage::new().embed(embed)).await?;
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    channel.delete(&ctx.http()).await?;
    if let Some(log_channel) = get_logging_channel(guild_id.into()) {
        let log_message = format!(
            "Ticket `{}` was closed by {}. Reason: {}",
            channel_name,
            ctx.author().mention(),
            reason
        );
        log_channel.say(&ctx.http(), log_message).await?;
    }
    Ok(())
}
