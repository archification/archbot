#![warn(clippy::str_to_string)]

mod commands;
mod utils;
mod config;
mod tickets;
mod staff;

use poise::serenity_prelude as serenity;
use std::{
    collections::HashMap,
    env::var,
    sync::{Arc, Mutex},
    time::Duration,
};
//use chrono::TimeZone;
use crate::utils::get_logging_channel;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    votes: Mutex<HashMap<String, u32>>,
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
    _data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::GuildMemberAddition { new_member } => {
            let guild_id = new_member.guild_id;
            if let Some(log_channel) = get_logging_channel(guild_id.into()) {
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
                log_channel.send_message(ctx, serenity::CreateMessage::new().embed(embed)).await?;
            }
        },
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            println!("Logged in as {}", data_about_bot.user.name);
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
            staff::ban(),
            commands::help(),
            commands::announce(),
            commands::vote(),
            commands::getvotes(),
            config::setlog(),
            config::setticket(),
            config::addticketrole(),
            config::removeticketrole(),
            config::setticketmessage(),
            config::setticketexemptrole(),
            config::removeticketexemptrole(),
            config::listticketroles(),
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
                Ok(Data {
                    votes: Mutex::new(HashMap::new()),
                })
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
