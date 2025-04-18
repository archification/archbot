use crate::{Context, Error};
use poise::serenity_prelude::{self as serenity, Mentionable};
use crate::utils::{get_logging_channel, LogEventType};

#[poise::command(
    prefix_command,
    track_edits,
    slash_command
)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "\nThe above commands are available within their respective scopes.",
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command
)]
pub async fn vote(
    ctx: Context<'_>,
    #[description = "What to vote for"] choice: String,
) -> Result<(), Error> {
    let num_votes = {
        let mut votes_map = ctx.data().votes.lock().await;
        let num_votes = votes_map.entry(choice.clone()).or_default();
        *num_votes += 1;
        *num_votes
    };
    let response = format!("Successfully voted for {choice}. {choice} now has {num_votes} votes!");
    ctx.say(response).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    track_edits,
    aliases("votes"),
    slash_command
)]
pub async fn getvotes(
    ctx: Context<'_>,
    #[description = "Choice to retrieve votes for"] choice: Option<String>,
) -> Result<(), Error> {
    let votes_map = ctx.data().votes.lock().await;
    if let Some(choice) = choice {
        let num_votes = votes_map.get(&choice).copied().unwrap_or(0);
        let response = match num_votes {
            0 => format!("Nobody has voted for {} yet", choice),
            _ => format!("{} people have voted for {}", num_votes, choice),
        };
        ctx.say(response).await?;
    } else {
        let mut response = String::new();
        if votes_map.is_empty() {
            response.push_str("Nobody has voted for anything yet :(");
        } else {
            for (choice, num_votes) in votes_map.iter() {
                response.push_str(&format!("{}: {} votes\n", choice, num_votes));
            }
        }
        ctx.say(response).await?;
    };
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
    #[description = "Message to announce"]
    #[rest]
    message: String,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let announcer = ctx.author();
    let target_channel = get_logging_channel(guild_id.into(), LogEventType::Announcements)
        .or_else(|| get_logging_channel(guild_id.into(), LogEventType::Default));
    match target_channel {
        Some(channel_id) => {
            let embed = serenity::CreateEmbed::new()
                .title("📢 Announcement")
                .description(&message)
                .color(serenity::Colour::GOLD);
            channel_id.send_message(
                &ctx.http(),
                serenity::CreateMessage::new()
                    .embed(embed)
            ).await?;
            if let Some(log_channel) = get_logging_channel(guild_id.into(), LogEventType::Moderation) {
                let log_embed = serenity::CreateEmbed::new()
                    .title("Announcement Log")
                    .description(format!("{}", channel_id.mention()))
                    .field("Content", &message, false)
                    .field("Announcer", format!("{}", announcer.mention()), true)
                    .color(serenity::Colour::DARK_GOLD);
                log_channel.send_message(
                    &ctx.http(),
                    serenity::CreateMessage::new()
                        .content("📢 Announcement created")
                        .embed(log_embed)
                ).await?;
            }
            ctx.say("✅ Announcement successfully sent!").await?;
        }
        None => {
            ctx.say("❌ No announcement channel configured for this server.\nUse `/config set_announcement_channel` to set one.").await?;
        }
    }
    Ok(())
}
