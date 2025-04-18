#![warn(clippy::str_to_string)]

mod cluster;
mod commands;
mod utils;
mod config;
mod tickets;
mod staff;

use poise::serenity_prelude as serenity;
use poise::serenity_prelude::ChannelId;
use tokio::sync::Mutex;
use std::{
    collections::HashMap,
    env::var,
    sync::Arc,
    time::Duration,
};
use crate::utils::get_logging_channels;
use crate::cluster::ClusterState;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Clone)]
pub struct Data {
    votes: Arc<Mutex<HashMap<String, u32>>>,
    cluster_state: Arc<Mutex<ClusterState>>,
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
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
    match event {
        serenity::FullEvent::Message { new_message } => {
            cluster::handle_cluster_message(ctx, new_message, data.cluster_state.clone(), Arc::new(Mutex::new(data.clone()))).await?;
        }
        serenity::FullEvent::GuildMemberAddition { new_member } => {
            let guild_id = new_member.guild_id;
            if let Some(log_channel) = crate::utils::get_logging_channel(
                guild_id.into(),
                crate::utils::LogEventType::MemberJoinLeave
            ) {
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
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            println!("Bot has started as {}", data_about_bot.user.name);
        },
        _ => {}
    }
    Ok(())
}

#[tokio::main]
async fn main() {
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
/*
        command_check: Some(|ctx| {
            Box::pin(async move {
                if ctx.author().id == 204537370224099328 {
                    return Ok(true);
                }
                Ok(false)
            })
        }),
*/
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
                        println!("Failed to load cluster config: {}. Using random instance ID and default priority", e);
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
                let logging_channels = get_logging_channels();
                for (guild_id_str, channel_id) in logging_channels {
                    if let Ok(guild_id) = guild_id_str.parse::<u64>() {
                        let channel = ChannelId::new(channel_id as u64);
                        let embed = serenity::CreateEmbed::new()
                            .title("Bot Online")
                            .description("The bot has started successfully!")
                            .color(serenity::Colour::DARK_GREEN);
                        if let Err(e) = channel.send_message(ctx, serenity::CreateMessage::new().embed(embed)).await {
                            println!("Failed to send boot announcement to guild {}: {}", guild_id, e);
                        }
                    }
                }
                Ok(data)
            })
        })
        .options(options)
        .build();
    let token = var("DISCORD_TOKEN")
        .expect("Missing `DISCORD_TOKEN` env var, see README for more information.");
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT | serenity::GatewayIntents::GUILD_MEMBERS;
    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap()
}
