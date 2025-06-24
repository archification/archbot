#![warn(clippy::str_to_string)]

mod cluster;
mod commands;
mod utils;
mod config;
mod tickets;
mod staff;

use poise::serenity_prelude as serenity;
use poise::serenity_prelude::ChannelId;
use poise::serenity_prelude::Mentionable;
use tokio::sync::Mutex;
use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
    env,
};
use clap::Parser;
use crate::utils::get_logging_channels;
use crate::cluster::ClusterState;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    dauth: Option<String>,
    #[arg(short, long)]
    coordination: Option<u64>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Clone)]
pub struct Data {
    votes: Arc<Mutex<HashMap<String, u32>>>,
    cluster_state: Arc<Mutex<ClusterState>>,
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {error}"),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {e}")
            }
        }
    }
}

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    let is_leader = data.cluster_state.lock().await.is_leader;
    match event {
        serenity::FullEvent::ReactionAdd { add_reaction } => {
            if add_reaction.user_id == Some(_framework.bot_id) {
                return Ok(());
            }
            if !is_leader {
                return Ok(());
            }
            if let (Some(guild_id), Some(user_id)) = (add_reaction.guild_id, add_reaction.user_id) {
                let emoji_str = add_reaction.emoji.to_string();
                if let Some(role_id) = crate::utils::get_react_role(
                    guild_id.into(),
                    add_reaction.message_id.into(),
                    &emoji_str
                ).await {
                    let member = guild_id.member(&ctx.http, user_id).await?;
                    if let Err(e) = member.add_role(&ctx.http, role_id).await {
                        println!("Failed to add role {} to user {}: {}", &role_id, &user_id, e);
                    }
                }
            }
        }
        serenity::FullEvent::ReactionRemove { removed_reaction } => {
            if removed_reaction.user_id == Some(_framework.bot_id) {
                return Ok(());
            }
            if !is_leader {
                return Ok(());
            }
            if let (Some(guild_id), Some(user_id)) = (removed_reaction.guild_id, removed_reaction.user_id) {
                let emoji_str = removed_reaction.emoji.to_string();
                if let Some(role_id) = crate::utils::get_react_role(
                    guild_id.into(),
                    removed_reaction.message_id.into(),
                    &emoji_str
                ).await {
                    let member = guild_id.member(&ctx.http, user_id).await?;
                    if let Err(e) = member.remove_role(&ctx.http, role_id).await {
                         println!("Failed to remove role {} from user {}: {}", &role_id, &user_id, &e);
                    }
                }
            }
        }
        serenity::FullEvent::Message { new_message } => {
            cluster::handle_cluster_message(ctx, new_message, data.cluster_state.clone(), Arc::new(Mutex::new(data.clone()))).await?;
        }
        serenity::FullEvent::GuildMemberAddition { new_member } => {
            let guild_id = new_member.guild_id;
            if let Some(log_channel) = crate::utils::get_logging_channel(
                guild_id.into(),
                crate::utils::LogEventType::MemberJoinLeave
            ).await {
                let user = &new_member.user;
                let account_age = chrono::Utc::now().signed_duration_since(*user.created_at());
                let account_age_days = account_age.num_days();
                let embed = serenity::CreateEmbed::new()
                    .title("New Member Joined")
                    .thumbnail(user.face())
                    .field("Username", format!("{} ({})", user.tag(), user.id), true)
                    .field("Account Created", format!(
                        "<t:{}:D> ({} days ago)",
                        user.created_at().unix_timestamp(),
                        account_age_days
                    ), true)
                    .field("Is Bot", user.bot.to_string(), true)
                    .color(serenity::Colour::DARK_GREEN);
                if let Some(guild) = new_member.guild_id.to_guild_cached(ctx) {
                    let _ = embed.clone().field("Member Count", guild.member_count.to_string(), true);
                }
                log_channel.send_message(ctx, serenity::CreateMessage::new().embed(embed)).await?;
            }
        },
        serenity::FullEvent::GuildMemberRemoval { guild_id, user, .. } => {
            let guild_id_u64 = <poise::serenity_prelude::GuildId as std::convert::Into<u64>>::into(*guild_id);
            if let Some(log_channel) = crate::utils::get_logging_channel(
                guild_id_u64,
                crate::utils::LogEventType::MemberJoinLeave
            ).await {
                let joined_at = if let Some(guild) = guild_id.to_guild_cached(ctx) {
                    guild.members.get(&user.id).and_then(|m| m.joined_at)
                } else {
                    None
                };
                let embed = serenity::CreateEmbed::new()
                    .title("Member Left")
                    .thumbnail(user.face())
                    .field("Username", format!("{} ({})", user.tag(), user.id), true)
                    .field("Joined At", match joined_at {
                        Some(joined_at) => format!("<t:{}:D>", joined_at.unix_timestamp()),
                        None => "Unknown".to_owned().to_string(),
                    }, true)
                    .field("Is Bot", user.bot.to_string(), true)
                    .color(serenity::Colour::DARK_RED);
                if let Some(guild) = guild_id.to_guild_cached(ctx) {
                    let _ = embed.clone().field("Member Count", guild.member_count.to_string(), true);
                }
                log_channel.send_message(ctx, serenity::CreateMessage::new().embed(embed)).await?;
            }
        },
        serenity::FullEvent::MessageDelete { channel_id, deleted_message_id, guild_id: Some(guild_id) } => {
            let guild_id_u64 = <poise::serenity_prelude::GuildId as std::convert::Into<u64>>::into(*guild_id);
            if let Some(log_channel) = crate::utils::get_logging_channel(
                guild_id_u64,
                crate::utils::LogEventType::MessageDeletion
            ).await {
                let mut embed = serenity::CreateEmbed::new()
                    .title("Message Deleted")
                    .description(format!("Message deleted in {}", channel_id.mention()))
                    .color(serenity::Colour::DARK_ORANGE);
                let (author_mention, content) = {
                    let cached_message = ctx.cache.message(*channel_id, *deleted_message_id);
                    if let Some(msg) = cached_message {
                        (Some(msg.author.mention().to_string()), Some(msg.content.clone()))
                    } else {
                        (None, None)
                    }
                };
                if let Some(author_mention) = author_mention {
                    embed = embed.field("Author", author_mention, true);
                }
                if let Some(content) = content {
                    embed = embed.field("Content", content, false);
                } else {
                    embed = embed.field("Message ID", deleted_message_id.to_string(), true);
                }
                log_channel.send_message(ctx, serenity::CreateMessage::new().embed(embed)).await?;
            }
        },
        serenity::FullEvent::MessageDeleteBulk { channel_id, multiple_deleted_messages_ids, guild_id: Some(guild_id) } => {
            let guild_id_u64 = <poise::serenity_prelude::GuildId as std::convert::Into<u64>>::into(*guild_id);
            if let Some(log_channel) = crate::utils::get_logging_channel(
                guild_id_u64,
                crate::utils::LogEventType::MessageDeletion
            ).await {
                let embed = serenity::CreateEmbed::new()
                    .title("Bulk Message Deletion")
                    .description(format!(
                        "{}, messages deleted in {}",
                        multiple_deleted_messages_ids.len(),
                        channel_id.mention()
                    ))
                    .color(serenity::Colour::DARK_ORANGE);
                log_channel.send_message(ctx, serenity::CreateMessage::new().embed(embed)).await?;
            }
        },
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            println!("Bot has started as {}", data_about_bot.user.name);
        },
        _ => {}
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let token = args.dauth
        .or_else(|| env::var("DISCORD_TOKEN").ok())
        .expect("Missing Discord token. Please set either DISCORD_TOKEN environment variable or use --dauth argument");
    let coordination_channel_id = args.coordination
        .or_else(|| env::var("COORDINATION_CHANNEL_ID").ok().and_then(|s| s.parse().ok()))
        .expect("Missing coordination channel ID. Please set either COORDINATION_CHANNEL_ID environment variable or use --coordination argument");
    let options = poise::FrameworkOptions {
        commands: vec![
            staff::quit(),
            staff::writeconfig(),
            staff::ban(),
            staff::kick(),
            commands::help(),
            commands::announce(),
            commands::vote(),
            commands::getvotes(),
            commands::diceroll(),
            config::config(),
            tickets::ticket(),
            tickets::closeticket(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("~".into()),
            edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                Duration::from_secs(3600),
            ))),
            additional_prefixes: vec![
                poise::Prefix::Literal("hey bot,"),
                poise::Prefix::Literal("hey bot"),
            ],
            ..Default::default()
        },
        on_error: |error| Box::pin(on_error(error)),
        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        post_command: |ctx| {
            Box::pin(async move {
                println!("Executed command {}!", ctx.command().qualified_name);
            })
        },
        skip_checks_for_owners: false,
        event_handler: |ctx, event, framework, data| {
            Box::pin(event_handler(ctx, event, framework, data))
        },
        ..Default::default()
    };
    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                let cluster_config = match crate::utils::load_cluster_config() {
                    Ok(config) => config,
                    Err(e) => {
                        println!("Failed to load cluster config: {e}. Using random instance ID and default priority");
                        crate::utils::ClusterConfig {
                            cluster: crate::utils::ClusterInfo {
                                instance_id: format!("instance-{}", rand::random::<u64>()),
                                priority: 1,
                            }
                        }
                    }
                };
                let cluster_state = Arc::new(Mutex::new(ClusterState::new(
                    cluster_config.cluster.instance_id,
                    cluster_config.cluster.priority,
                    coordination_channel_id,
                )));
                let data = Data {
                    votes: Arc::new(Mutex::new(HashMap::new())),
                    cluster_state: cluster_state.clone(),
                };
                let ctx_for_cluster = ctx.clone();
                let data_for_cluster = Arc::new(Mutex::new(data.clone()));
                tokio::spawn(async move {
                    cluster::start_cluster_loop(
                        ctx_for_cluster,
                        data_for_cluster,
                        cluster_state.clone()
                    ).await;
                });
                let logging_channels = get_logging_channels().await;
                for (guild_id_str, channel_id) in logging_channels {
                    if let Ok(guild_id) = guild_id_str.parse::<u64>() {
                        let channel = ChannelId::new(channel_id as u64);
                        let cluster_state = data.cluster_state.lock().await;
                        let something = format!("Instance {} has started successfully!", cluster_state.my_instance_id);
                        let embed = serenity::CreateEmbed::new()
                            .title("Instance Online")
                            .description(something)
                            .color(serenity::Colour::DARK_GREEN);
                        if let Err(e) = channel.send_message(ctx, serenity::CreateMessage::new().embed(embed)).await {
                            println!("Failed to send boot announcement to guild {guild_id}: {e}");
                        }
                    }
                }
                Ok(data)
            })
        })
        .options(options)
        .build();
    let intents =
        serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_MEMBERS
        | serenity::GatewayIntents::GUILD_MESSAGE_REACTIONS;
    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap()
}
