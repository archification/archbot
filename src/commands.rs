use crate::{Context, Error};
use poise::serenity_prelude::GuildId;
use poise::serenity_prelude::Permissions;
use poise::serenity_prelude::PermissionOverwrite;
use poise::serenity_prelude::{self as serenity, CreateChannel, CreateEmbed, CreateMessage};
use poise::serenity_prelude::PermissionOverwriteType;
use poise::serenity_prelude::Mentionable;
use poise::serenity_prelude::ChannelId;
use toml::Value;
use std::collections::HashMap;
use std::fs;
use std::io::Write;

const CONFIG_PATH: &str = "/home/jaster/wut/rs/archbot/config.toml";

fn get_logging_channel(guild_id: u64) -> Option<ChannelId> {
    let toml_content = fs::read_to_string(CONFIG_PATH)
        .expect("Failed to read config file");
    let value = toml_content.parse::<Value>().expect("Failed to parse TOML");
    let guild_section = value.get(guild_id.to_string())
        .and_then(|v| v.as_table())?;
    let channel_id = guild_section.get("logging_channel")
        .and_then(|v| v.as_integer())?;
    Some(ChannelId::new(channel_id as u64))
}

fn get_ticket_category(guild_id: u64) -> Option<serenity::ChannelId> {
    let toml_content = fs::read_to_string(CONFIG_PATH)
        .expect("Failed to read config file");
    let value = toml_content.parse::<Value>().expect("Failed to parse TOML");
    value.get(guild_id.to_string())
        .and_then(|v| v.as_table())
        .and_then(|guild_table| guild_table.get("ticket_category"))
        .and_then(|v| v.as_integer())
        .map(|category_id| serenity::ChannelId::new(category_id as u64))
}

#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "This is an example bot made to showcase features of my custom Discord bot framework",
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only)]
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
    let channel_builder = CreateChannel::new(channel_name)
        .kind(serenity::ChannelType::Text)
        .category(category_id)
        .topic(issue_description.clone())
        .permissions(vec![
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
        ]);
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

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    category = "Moderation",
    guild_only
)]
pub async fn setlog(
    ctx: Context<'_>,
    #[description = "Channel to send logs to"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let config_path = CONFIG_PATH;
    let toml_content = fs::read_to_string(config_path)?;
    let mut value = toml_content.parse::<Value>().expect("Failed to parse TOML");
    let guild_table = value
        .as_table_mut()
        .expect("Root should be a table")
        .entry(guild_id.to_string())
        .or_insert(Value::Table(toml::value::Table::new()))
        .as_table_mut()
        .expect("Guild section should be a table");
    guild_table.insert("logging_channel".to_string(), Value::Integer(channel.id.into()));
    let new_toml = toml::to_string_pretty(&value)?;
    let mut file = fs::File::create(config_path)?;
    file.write_all(new_toml.as_bytes())?;
    ctx.say(format!("Updated logging channel to {}", channel.name)).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    category = "Moderation",
    guild_only
)]
pub async fn setticket(
    ctx: Context<'_>,
    #[description = "Category to use for tickets"]
    #[channel_types("Category")]
    channel: serenity::GuildChannel,
) -> Result<(), Error> {
    if channel.kind != serenity::ChannelType::Category {
        ctx.say("Please select a category channel, not a regular channel").await?;
        return Ok(());
    }
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let toml_content = fs::read_to_string(CONFIG_PATH)?;
    let mut value = toml_content.parse::<Value>().expect("Failed to parse TOML");
    let guild_table = value
        .as_table_mut()
        .expect("Root should be a table")
        .entry(guild_id.to_string())
        .or_insert(Value::Table(toml::value::Table::new()))
        .as_table_mut()
        .expect("Guild section should be a table");
    guild_table.insert("ticket_category".to_string(), Value::Integer(channel.id.into()));
    let new_toml = toml::to_string_pretty(&value)?;
    let mut file = fs::File::create(CONFIG_PATH)?;
    file.write_all(new_toml.as_bytes())?;
    ctx.say(format!("Updated ticket category to {}", channel.name)).await?;
    Ok(())
}

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn quit(ctx: Context<'_>) -> Result<(), Error> {
    let response = "Shutting down!";
    ctx.say(response).await?;
    ctx.framework().shard_manager().shutdown_all().await;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn vote(
    ctx: Context<'_>,
    #[description = "What to vote for"] choice: String,
) -> Result<(), Error> {
    let num_votes = {
        let mut hash_map = ctx.data().votes.lock().unwrap();
        let num_votes = hash_map.entry(choice.clone()).or_default();
        *num_votes += 1;
        *num_votes
    };
    let response = format!("Successfully voted for {choice}. {choice} now has {num_votes} votes!");
    ctx.say(response).await?;
    Ok(())
}

#[poise::command(prefix_command, track_edits, aliases("votes"), slash_command)]
pub async fn getvotes(
    ctx: Context<'_>,
    #[description = "Choice to retrieve votes for"] choice: Option<String>,
) -> Result<(), Error> {
    if let Some(choice) = choice {
        let num_votes = *ctx.data().votes.lock().unwrap().get(&choice).unwrap_or(&0);
        let response = match num_votes {
            0 => format!("Nobody has voted for {} yet", choice),
            _ => format!("{} people have voted for {}", num_votes, choice),
        };
        ctx.say(response).await?;
    } else {
        let mut response = String::new();
        for (choice, num_votes) in ctx.data().votes.lock().unwrap().iter() {
            response += &format!("{}: {} votes", choice, num_votes);
        }
        if response.is_empty() {
            response += "Nobody has voted for anything yet :(";
        }
        ctx.say(response).await?;
    };
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "BAN_MEMBERS",
    category = "Moderation",
    guild_only
)]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "User to ban"] user: serenity::User,
    #[description = "Reason for ban"] reason: Option<String>,
    #[description = "Days of messages to delete (0-7)"]
    #[min = 0]
    #[max = 7]
    delete_message_days: Option<u8>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let reason = reason.unwrap_or_else(|| "No reason provided".to_owned());
    let delete_message_days = delete_message_days.unwrap_or(0);
    guild_id.ban_with_reason(&ctx.http(), user.id, delete_message_days, &reason).await?;
    let response = format!(
        "🔨 Banned {} ({}) | Reason: {}",
        user.name, user.id, reason
    );
    ctx.say(&response).await?;
    if let Some(target_channel_id) = get_logging_channel(guild_id.into()) {
        target_channel_id.say(&ctx.http(), response).await?;
    }
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    category = "Moderation",
    guild_only
)]
pub async fn announce(
    ctx: Context<'_>,
    #[description = "Message to announce"] message: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let guild_id_num: u64 = guild_id.into();
    println!("Looking up channel for guild ID: {}", guild_id_num);
    if let Some(target_channel_id) = get_logging_channel(guild_id_num) {
        println!("Found channel ID: {}", target_channel_id);
        target_channel_id.say(&ctx.http(), message).await?;
        ctx.say("Announcement sent!").await?;
    } else {
        println!("Available guild IDs in config: {:?}",
            get_logging_channels().keys().collect::<Vec<_>>());
        ctx.say("No announcement channel configured for this server.").await?;
    }
    Ok(())
}

fn get_logging_channels() -> HashMap<String, i64> {
    let toml_content = fs::read_to_string(CONFIG_PATH)
        .expect("Failed to read config file");
    let value = toml_content.parse::<Value>().expect("Failed to parse TOML");
    value.as_table()
        .map(|table| {
            table.iter()
                .filter_map(|(guild_id, v)| {
                    v.as_table()
                        .and_then(|guild_table| guild_table.get("logging_channel"))
                        .and_then(|v| v.as_integer())
                        .map(|channel_id| (guild_id.clone(), channel_id))
                })
                .collect()
        })
        .unwrap_or_default()
}
