use crate::{Context, Error};
use poise::serenity_prelude::GuildId;
use poise::serenity_prelude::{self as serenity, CreateChannel, CreateEmbed, CreateMessage};
use poise::serenity_prelude::{Permissions, PermissionOverwrite, PermissionOverwriteType};
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
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let author = ctx.author();
    let category_id = match get_ticket_category(guild_id.into()) {
        Some(id) => id,
        None => {
            ctx.say("❌ Ticket system not configured. Admins must set a ticket category first.").await?;
            return Ok(());
        }
    };
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let channel_name = format!("ticket-{}-{}", author.name.to_lowercase().replace(' ', "-"), timestamp);
    let issue_description = issue.unwrap_or_else(|| "No description provided".to_string());
    let mut permissions = vec![
        PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES |
                   Permissions::EMBED_LINKS | Permissions::ATTACH_FILES |
                   Permissions::READ_MESSAGE_HISTORY,
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
            allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES |
                   Permissions::MANAGE_MESSAGES,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Role(serenity::RoleId::new(role_id)),
        });
    }
    let channel = guild_id.create_channel(&ctx.http(),
        CreateChannel::new(&channel_name)
            .kind(serenity::ChannelType::Text)
            .category(category_id)
            .topic(&issue_description)
            .permissions(permissions)
    ).await?;
    channel.say(&ctx.http(), format!("{} created this ticket", author.mention())).await?;
    let show_message = match get_ticket_exempt_role(guild_id.into()) {
        Some(exempt_role_id) => {
            let member = guild_id.member(&ctx.http(), author.id).await?;
            !member.roles.contains(&serenity::RoleId::new(exempt_role_id))
        }
        None => true,
    };
    if show_message {
        let template_path = get_ticket_template_path(guild_id.into());
        let message = std::fs::read_to_string(&template_path)
            .unwrap_or_else(|_|
                "Thank you for creating a ticket! Support staff will be with you shortly.".to_string()
            );
        channel.say(&ctx.http(), message).await?;
    }
    let embed = serenity::CreateEmbed::new()
        .title("Ticket Created")
        .description(&issue_description)
        .field("Status", "Open", true)
        .field("Created By", author.mention().to_string(), true)
        .color(serenity::Colour::DARK_GREEN);
    channel.send_message(&ctx.http(),
        serenity::CreateMessage::new().embed(embed)
    ).await?;
    if let Some(log_channel) = get_logging_channel(guild_id.into(), LogEventType::TicketActivity) {
        let log_embed = serenity::CreateEmbed::new()
            .title("New Ticket Created")
            .description(format!("[Jump to Ticket]({})", channel.id.mention()))
            .field("Creator", format!("{} ({})", author.tag(), author.id), true)
            .field("Description", &issue_description, false)
            .color(serenity::Colour::DARK_GREEN);
        log_channel.send_message(&ctx.http(),
            serenity::CreateMessage::new()
                .content("📬 New ticket created!")
                .embed(log_embed)
        ).await?;
    }
    ctx.say(format!("✅ Created your ticket: {}", channel.mention())).await?;
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
    #[description = "Reason for closing"]
    #[rest]
    reason: Option<String>,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let channel = ctx.channel_id();
    let closer = ctx.author();
    let channel_info = channel.to_channel(&ctx.http()).await?;
    let channel_name = match &channel_info {
        serenity::Channel::Guild(guild_channel) => {
            if !guild_channel.name.starts_with("ticket-") {
                ctx.say("❌ This command can only be used in ticket channels").await?;
                return Ok(());
            }
            &guild_channel.name
        }
        _ => {
            ctx.say("❌ This command can only be used in server channels").await?;
            return Ok(());
        }
    };
    let reason = reason.unwrap_or_else(|| "No reason provided".to_string());
    let embed = CreateEmbed::new()
        .title("Ticket Closed")
        .description(&reason)
        .field("Closed By", closer.mention().to_string(), true)
        .field("Closed At", format!("<t:{}:F>", chrono::Utc::now().timestamp()), true)
        .color(serenity::Colour::DARK_RED);
    channel.send_message(&ctx.http(), CreateMessage::new().embed(embed)).await?;
    if let Some(log_channel) = get_logging_channel(guild_id.into(), LogEventType::TicketActivity) {
        let log_embed = CreateEmbed::new()
            .title("Ticket Closed")
            .description(format!("[Original Ticket]({})", channel.mention()))
            .field("Ticket Name", channel_name, true)
            .field("Closed By", format!("{} ({})", closer.tag(), closer.id), true)
            .field("Reason", &reason, false)
            .color(serenity::Colour::DARK_RED);
        log_channel.send_message(
            &ctx.http(),
            CreateMessage::new()
                .content("📪 Ticket closed")
                .embed(log_embed)
        ).await?;
    }
    ctx.say("🗑 Closing this ticket in 5 seconds...").await?;
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    match channel.delete(&ctx.http()).await {
        Ok(_) => {
            if let Some(log_channel) = get_logging_channel(guild_id.into(), LogEventType::TicketActivity) {
                log_channel.say(
                    &ctx.http(),
                    format!("✅ Successfully deleted ticket channel: `{channel_name}`")
                ).await?;
            }
        }
        Err(e) => {
            println!("Failed to delete ticket channel: {e}");
            if let Some(log_channel) = get_logging_channel(guild_id.into(), LogEventType::TicketActivity) {
                log_channel.say(
                    &ctx.http(),
                    format!("⚠️ Failed to delete ticket channel `{channel_name}`: {e}")
                ).await?;
            }
        }
    }
    Ok(())
}
