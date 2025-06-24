use crate::{Context, Error};
use poise::serenity_prelude::{self as serenity, Mentionable};
use poise::serenity_prelude::ChannelId;
use poise::serenity_prelude::parse_emoji;
use toml::Value;
use std::fs;
use crate::cluster::ClusterMessage;
use crate::utils::*;

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    category = "Config",
    guild_only,
    subcommands(
        "log_channel",
        "ticket_category",
        "add_ticket_role",
        "remove_ticket_role",
        "ticket_message",
        "ticket_exempt_role",
        "remove_ticket_exempt_role",
        "list_ticket_roles",
        "set_member_log_channel",
        "set_ticket_log_channel",
        "set_announcement_channel",
        "reactrole",
        "removereactrole",
    )
)]
pub async fn config(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    poise::builtins::help(
        ctx,
        None,
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "\nUse these subcommands to configure the bot.",
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn set_log_channel(
    ctx: Context<'_>,
    #[description = "Type of events to log"]
    #[rename = "type"]
    event_type: String,
    #[description = "Channel to send logs to"]
    channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let coordination_channel_id = cluster_state.coordination_channel_id;
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let channel_key = match event_type.to_lowercase().as_str() {
        "boot" | "shutdown" => "boot_quit_channel",
        "member" | "join" | "leave" => "member_log_channel",
        "ticket" => "ticket_log_channel",
        "announcement" => "announcement_channel",
        "mod" | "moderation" => "mod_log_channel",
        "message" | "deletion" => "message_log_channel",
        _ => "logging_channel",
    };
    let _ = set_specific_logging_channel(guild_id.into(), channel_key, channel.id.into()).await;
    ctx.say(format!("Updated {} channel to {}", channel_key, channel.name)).await?;
    let config_str = crate::utils::get_config_as_string().await?;
    let cluster_channel = ChannelId::new(coordination_channel_id);
    cluster_channel.send_message(
        &ctx.http(),
        serenity::CreateMessage::new()
            .content(serde_json::to_string(&ClusterMessage::ConfigUpdate(config_str))?)
    ).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn set_announcement_channel(
    ctx: Context<'_>,
    #[description = "Channel for announcements"]
    channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let coordination_channel_id = cluster_state.coordination_channel_id;
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let _ = crate::utils::set_specific_logging_channel(
        guild_id.into(),
        "announcement_channel",
        channel.id.into()
    ).await;
    ctx.say(format!("📢 Announcements will now be sent to {}", channel.mention())).await?;
    let config_str = crate::utils::get_config_as_string().await?;
    let cluster_channel = ChannelId::new(coordination_channel_id);
    cluster_channel.send_message(
        &ctx.http(),
        serenity::CreateMessage::new()
            .content(serde_json::to_string(&ClusterMessage::ConfigUpdate(config_str))?)
    ).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn set_ticket_log_channel(
    ctx: Context<'_>,
    #[description = "Channel to send ticket logs to"]
    channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let coordination_channel_id = cluster_state.coordination_channel_id;
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let _ = crate::utils::set_specific_logging_channel(
        guild_id.into(),
        "ticket_log_channel",
        channel.id.into()
    ).await;
    ctx.say(format!("Updated ticket log channel to {}", channel.name)).await?;
    let config_str = crate::utils::get_config_as_string().await?;
    let cluster_channel = ChannelId::new(coordination_channel_id);
    cluster_channel.send_message(
        &ctx.http(),
        serenity::CreateMessage::new()
            .content(serde_json::to_string(&ClusterMessage::ConfigUpdate(config_str))?)
    ).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn set_member_log_channel(
    ctx: Context<'_>,
    #[description = "Channel to send member join/leave logs to"]
    channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let coordination_channel_id = cluster_state.coordination_channel_id;
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let _ = crate::utils::set_specific_logging_channel(
        guild_id.into(),
        "member_log_channel",
        channel.id.into()
    ).await;
    ctx.say(format!("Updated member log channel to {}", channel.name)).await?;
    let config_str = crate::utils::get_config_as_string().await?;
    let cluster_channel = ChannelId::new(coordination_channel_id);
    cluster_channel.send_message(
        &ctx.http(),
        serenity::CreateMessage::new()
            .content(serde_json::to_string(&ClusterMessage::ConfigUpdate(config_str))?)
    ).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn log_channel(
    ctx: Context<'_>,
    #[description = "Channel to send logs to"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let coordination_channel_id = cluster_state.coordination_channel_id;
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let _ = set_logging_channel(guild_id.into(), channel.id.into()).await;
    ctx.say(format!("Updated logging channel to {}", channel.name)).await?;
    let config_str = crate::utils::get_config_as_string().await?;
    let cluster_channel = ChannelId::new(coordination_channel_id);
    cluster_channel.send_message(
        &ctx.http(),
        serenity::CreateMessage::new()
            .content(serde_json::to_string(&ClusterMessage::ConfigUpdate(config_str))?)
    ).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn ticket_category(
    ctx: Context<'_>,
    #[description = "Category to use for tickets"]
    #[channel_types("Category")]
    channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let coordination_channel_id = cluster_state.coordination_channel_id;
    if channel.kind != serenity::ChannelType::Category {
        ctx.say("Please select a category channel, not a regular channel").await?;
        return Ok(());
    }
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let _ = set_ticket_category(guild_id.into(), channel.id.into()).await;
    ctx.say(format!("Updated ticket category to {}", channel.name)).await?;
    let config_str = crate::utils::get_config_as_string().await?;
    let cluster_channel = ChannelId::new(coordination_channel_id);
    cluster_channel.send_message(
        &ctx.http(),
        serenity::CreateMessage::new()
            .content(serde_json::to_string(&ClusterMessage::ConfigUpdate(config_str))?)
    ).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn add_ticket_role(
    ctx: Context<'_>,
    #[description = "Role to add to ticket access"] role: serenity::Role,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let coordination_channel_id = cluster_state.coordination_channel_id;
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let _ = add_ticrole(guild_id.into(), role.id.into()).await;
    ctx.say(format!("Added {} to ticket access roles", role.name)).await?;
    let config_str = crate::utils::get_config_as_string().await?;
    let cluster_channel = ChannelId::new(coordination_channel_id);
    cluster_channel.send_message(
        &ctx.http(),
        serenity::CreateMessage::new()
            .content(serde_json::to_string(&ClusterMessage::ConfigUpdate(config_str))?)
    ).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn remove_ticket_role(
    ctx: Context<'_>,
    #[description = "Role to remove from ticket access"] role: serenity::Role,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let coordination_channel_id = cluster_state.coordination_channel_id;
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let _ = remove_ticrole(guild_id.into(), role.id.into()).await;
    ctx.say(format!("Removed {} from ticket access roles", role.name)).await?;
    let config_str = crate::utils::get_config_as_string().await?;
    let cluster_channel = ChannelId::new(coordination_channel_id);
    cluster_channel.send_message(
        &ctx.http(),
        serenity::CreateMessage::new()
            .content(serde_json::to_string(&ClusterMessage::ConfigUpdate(config_str))?)
    ).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn ticket_message(
    ctx: Context<'_>,
    #[description = "Text file containing the ticket message template"]
    file: serenity::Attachment,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let coordination_channel_id = cluster_state.coordination_channel_id;
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    if !file.filename.ends_with(".txt") {
        ctx.say("Please upload a .txt file").await?;
        return Ok(());
    }
    let content = file.download().await?;
    let content = String::from_utf8(content)?;
    std::fs::create_dir_all("./ticket_templates")?;
    let path = get_ticket_template_path(guild_id.into());
    std::fs::write(path, &content)?;
    let cluster_channel = ChannelId::new(coordination_channel_id);
    let message = ClusterMessage::TicketTemplateUpdate {
        guild_id: guild_id.into(),
        content: content.clone(),
    };
    if let Err(e) = cluster_channel.send_message(
        &ctx.http(),
        serenity::CreateMessage::new()
            .content(serde_json::to_string(&message)?)
    ).await {
        println!("Failed to send template update: {e}");
    }
    ctx.say("Ticket message template updated and synced across instances!").await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn ticket_exempt_role(
    ctx: Context<'_>,
    #[description = "Role that exempts users from seeing the ticket message"]
    role: serenity::Role,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let coordination_channel_id = cluster_state.coordination_channel_id;
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let _ = set_ticket_exempt_role(guild_id.into(), role.id.into()).await;
    ctx.say(format!("Set {} as the ticket exempt role", role.name)).await?;
    let config_str = crate::utils::get_config_as_string().await?;
    let cluster_channel = ChannelId::new(coordination_channel_id);
    cluster_channel.send_message(
        &ctx.http(),
        serenity::CreateMessage::new()
            .content(serde_json::to_string(&ClusterMessage::ConfigUpdate(config_str))?)
    ).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn remove_ticket_exempt_role(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let coordination_channel_id = cluster_state.coordination_channel_id;
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let toml_content = fs::read_to_string(CONFIG_PATH)?;
    let mut value = toml_content.parse::<Value>().expect("Failed to parse TOML");
    if let Some(guild_table) = value
        .as_table_mut()
        .expect("Root should be a table")
        .get_mut(&guild_id.to_string())
        .and_then(|v| v.as_table_mut())
    {
        guild_table.remove("ticket_exempt_role");
    }
    let new_toml = toml::to_string_pretty(&value)?;
    fs::write(CONFIG_PATH, new_toml)?;
    ctx.say("Removed ticket exempt role").await?;
    let config_str = crate::utils::get_config_as_string().await?;
    let cluster_channel = ChannelId::new(coordination_channel_id);
    cluster_channel.send_message(
        &ctx.http(),
        serenity::CreateMessage::new()
            .content(serde_json::to_string(&ClusterMessage::ConfigUpdate(config_str))?)
    ).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn list_ticket_roles(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let roles = get_ticket_roles(guild_id.into()).await;
    if roles.is_empty() {
        ctx.say("No ticket access roles configured").await?;
        return Ok(());
    }
    let mut response = "Ticket access roles:\n".to_string();
    for role_id in roles {
        if let Some(role) = ctx.guild().unwrap().roles.get(&serenity::RoleId::new(role_id)) {
            response.push_str(&format!("- {}\n", role.name));
        }
    }
    ctx.say(response).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn reactrole(
    ctx: Context<'_>,
    #[description = "The channel where the message is located"]
    channel: serenity::GuildChannel,
    #[description = "The ID of the message to add the react-role to"]
    message_id: String,
    #[description = "The role to assign"]
    role: serenity::Role,
    #[description = "The emoji to react with"]
    emoji: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        ctx.say("This command can only be run by the leader instance.").await?;
        return Ok(());
    }
    let coordination_channel_id = cluster_state.coordination_channel_id;
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let message_id_u64 = match message_id.parse::<u64>() {
        Ok(id) => id,
        Err(_) => {
            ctx.say("❌ Invalid message ID. Please provide a valid message ID number.").await?;
            return Ok(());
        }
    };
    let message = match channel.id.message(&ctx.http(), message_id_u64).await {
        Ok(msg) => msg,
        Err(_) => {
            ctx.say("❌ Could not find the specified message in that channel. Make sure the ID is correct and I have permissions to view the channel.").await?;
            return Ok(());
        }
    };
    let reaction: serenity::ReactionType = if let Some(emoji_id) = parse_emoji(emoji.clone()) {
        emoji_id.into()
    } else {
        serenity::ReactionType::Unicode(emoji.clone())
    };
    if (message.react(&ctx.http(), reaction.clone()).await).is_err() {
        ctx.say("❌ Failed to react to the message. Do I have 'Add Reactions' permissions in that channel? Also, ensure the emoji is correct and I have access to it if it's a custom emoji from another server.").await?;
        return Ok(());
    }
/*
    if let Err(_) = message.react(&ctx.http(), reaction).await {
        ctx.say("❌ Failed to react to the message. Do I have 'Add Reactions' permissions in that channel? Also, ensure the emoji is correct and I have access to it if it's a custom emoji from another server.").await?;
        return Ok(());
    }
*/
    add_react_role(guild_id.into(), message_id_u64, emoji.clone(), role.id.into()).await?;
    ctx.say(format!(
        "✅ React-role configured. Users who react with {} on that message will now get the {} role.",
        emoji,
        role.name
    )).await?;
    let config_str = crate::utils::get_config_as_string().await?;
    let cluster_channel = ChannelId::new(coordination_channel_id);
    cluster_channel.send_message(
        &ctx.http(),
        serenity::CreateMessage::new()
            .content(serde_json::to_string(&ClusterMessage::ConfigUpdate(config_str))?)
    ).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn removereactrole(
    ctx: Context<'_>,
    #[description = "The channel where the message is located"]
    channel: serenity::GuildChannel,
    #[description = "The ID of the message to remove the react-role from"]
    message_id: String,
    #[description = "The emoji of the react-role to remove"]
    emoji: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        ctx.say("This command can only be run by the leader instance.").await?;
        return Ok(());
    }
    let coordination_channel_id = cluster_state.coordination_channel_id;
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let message_id_u64 = match message_id.parse::<u64>() {
        Ok(id) => id,
        Err(_) => {
            ctx.say("❌ Invalid message ID.").await?;
            return Ok(());
        }
    };
    match remove_react_role(guild_id.into(), message_id_u64, &emoji).await? {
        Some(_removed_role_id) => {
            let message = channel.id.message(&ctx.http(), message_id_u64).await?;
            let reaction_emoji: serenity::ReactionType = poise::serenity_prelude::parse_emoji(emoji.clone())
                .ok_or("Invalid custom emoji format.")?
                .into();
            let bot_id = ctx.framework().bot_id;
            message.delete_reaction(&ctx.http(), Some(bot_id), reaction_emoji).await?;
            ctx.say(format!("✅ Successfully removed the react-role for {}.", &emoji)).await?;
            let config_str = crate::utils::get_config_as_string().await?;
            let cluster_channel = ChannelId::new(coordination_channel_id);
            cluster_channel.send_message(
                &ctx.http(),
                serenity::CreateMessage::new()
                    .content(serde_json::to_string(&ClusterMessage::ConfigUpdate(config_str))?)
            ).await?;
        }
        None => {
            ctx.say("❌ That emoji was not configured as a react-role on that message.").await?;
        }
    }
    Ok(())
}
