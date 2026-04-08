use crate::{Context, Error};
use poise::serenity_prelude::{self as serenity, Mentionable};
use crate::utils::{get_logging_channel, LogEventType};
use rand::Rng;
use rand::seq::IndexedRandom;
use serde_json::Value;
use std::env;

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
            0 => format!("Nobody has voted for {choice} yet"),
            _ => format!("{num_votes} people have voted for {choice}"),
        };
        ctx.say(response).await?;
    } else {
        let mut response = String::new();
        if votes_map.is_empty() {
            response.push_str("Nobody has voted for anything yet :(");
        } else {
            for (choice, num_votes) in votes_map.iter() {
                response.push_str(&format!("{choice}: {num_votes} votes\n"));
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
    let mut target_channel = get_logging_channel(guild_id.into(), LogEventType::Announcements).await;
    if target_channel.is_none() {
        target_channel = get_logging_channel(guild_id.into(), LogEventType::Default).await;
    }
    /*
    let target_channel = get_logging_channel(guild_id.into(), LogEventType::Announcements).await
        .or_else(|| get_logging_channel(guild_id.into(), LogEventType::Default).await);
    */
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
            if let Some(log_channel) = get_logging_channel(guild_id.into(), LogEventType::Moderation).await {
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

#[poise::command(
    prefix_command,
    slash_command,
    aliases("dice", "roll"),
    category = "Fun",
    track_edits
)]
pub async fn diceroll(
    ctx: Context<'_>,
    #[description = "Dice to roll in XdY format with optional operations (e.g., 2d6*5, 3d7+2)"]
    dice: Option<String>,
) -> Result<(), Error> {
    let input = dice.as_deref().unwrap_or("1d6");
    let (dice_part, operations) = parse_dice_expression(input);
    let parts: Vec<&str> = dice_part.split('d').collect();
    if parts.len() != 2 {
        ctx.say("❌ Invalid dice format. Please use XdY format (e.g., 1d6, 2d20) with optional operations (*, /, +, -)").await?;
        return Ok(());
    }
    let num_dice = parts[0].parse::<i32>().unwrap_or(1);
    let num_sides = parts[1].parse::<i32>().unwrap_or(6);
    if num_dice <= 0 || num_sides <= 0 {
        ctx.say("❌ Number of dice and sides must be greater than 0").await?;
        return Ok(());
    }
    if num_dice > 100 {
        ctx.say("❌ Maximum number of dice is 100").await?;
        return Ok(());
    }
    if num_sides > 1000 {
        ctx.say("❌ Maximum number of sides is 1000").await?;
        return Ok(());
    }
    let rolls: Vec<i32> = (0..num_dice)
        .map(|_| {
            let mut rng = rand::rng();
            rng.random_range(1..=num_sides)
        })
        .collect();
    let mut total: i32 = rolls.iter().sum();
    let operations_str = if let Some(ops) = &operations {
        let original_total = total;
        for op in ops {
            match op {
                Operator::Add(val) => total += val,
                Operator::Subtract(val) => total -= val,
                Operator::Multiply(val) => total *= val,
                Operator::Divide(val) => {
                    if *val == 0 {
                        ctx.say("❌ Division by zero is not allowed").await?;
                        return Ok(());
                    }
                    total /= val;
                }
            }
        }
        format!(" (after operations: {original_total} → {total})")
    } else {
        String::new()
    };
    let response = if num_dice == 1 {
        format!("🎲 You rolled **{}** (1d{}){}", rolls[0], num_sides, operations_str)
    } else {
        let rolls_str = rolls.iter()
            .map(|r| r.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        format!(
            "🎲 You rolled **{total}** ({num_dice}d{num_sides}) - Individual rolls: {rolls_str}{operations_str}"
        )
    };
    ctx.say(response).await?;
    Ok(())
}

#[derive(Debug)]
enum Operator {
    Add(i32),
    Subtract(i32),
    Multiply(i32),
    Divide(i32),
}

fn parse_dice_expression(input: &str) -> (&str, Option<Vec<Operator>>) {
    let mut operators = Vec::new();
    let dice_end = input.find(|c: char| "+-*/".contains(c)).unwrap_or(input.len());
    let dice_part = &input[..dice_end];
    let mut chars = input[dice_end..].chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '+' | '-' | '*' | '/' => {
                let op = c;
                let mut num_str = String::new();
                if c == '-' && chars.peek().is_some_and(|&ch| ch == '-') {
                    num_str.push('-');
                    chars.next();
                }
                while let Some(&next_char) = chars.peek() {
                    if next_char.is_ascii_digit() {
                        num_str.push(next_char);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if !num_str.is_empty() {
                    if let Ok(num) = num_str.parse::<i32>() {
                        match op {
                            '+' => operators.push(Operator::Add(num)),
                            '-' => operators.push(Operator::Subtract(num)),
                            '*' => operators.push(Operator::Multiply(num)),
                            '/' => operators.push(Operator::Divide(num)),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
    (dice_part, if operators.is_empty() { None } else { Some(operators) })
}

#[derive(poise::ChoiceParameter)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[poise::command(slash_command, prefix_command, category = "Fun")]
pub async fn countdown(
    ctx: Context<'_>,
    #[description = "Starting number (default 10)"] start: Option<u32>,
    #[description = "Difficulty (easy, medium, hard)"] difficulty: Option<Difficulty>,
    #[description = "Ending name (or 'random')"] ending: Option<String>,
) -> Result<(), Error> {
    let start_val = start.unwrap_or(10);
    if start_val == 0 || start_val > 100 {
        ctx.say("❌ Starting number must be between 1 and 100.").await?;
        return Ok(());
    }
    let diff = difficulty.unwrap_or(Difficulty::Easy);
    let max_chance = match diff {
        Difficulty::Easy => 0.0,
        Difficulty::Medium => 0.25,
        Difficulty::Hard => 0.50,
    };
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let endings = crate::utils::get_countdown_endings(guild_id.into()).await;
    let final_message = if let Some(ref end_name) = ending {
        if end_name.to_lowercase() == "random" && !endings.is_empty() {
            let mut rng = rand::rng();
            let values: Vec<&String> = endings.values().collect();
            values.choose(&mut rng).map(|s| s.to_string()).unwrap_or_else(|| "0".to_string())
        } else if let Some(msg) = endings.get(&end_name.to_lowercase()) {
            msg.clone()
        } else {
            "0".to_string()
        }
    } else {
        "0".to_string()
    };
    let mut current = start_val as i32;
    let msg = ctx.say(format!("⏱️ {}", current)).await?;
    while current > 0 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let progress = if current <= start_val as i32 {
            (start_val as f32 - current as f32) / (start_val as f32)
        } else {
            0.0
        };
        let chance_to_go_up = max_chance * progress;
        let roll: f32 = {
            let mut rng = rand::rng();
            rng.random()
        };
        if roll < chance_to_go_up {
            current += 1;
        } else {
            current -= 1;
        }
        let content_to_send = if current == 0 {
            final_message.clone()
        } else {
            format!("⏱️ {}", current)
        };
        if let Err(e) = msg.edit(ctx, poise::CreateReply::default().content(content_to_send)).await {
            println!("Failed to edit countdown message: {}", e);
            break;
        }
    }
    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    category = "Fun",
    nsfw_only
)]
pub async fn reddit(
    ctx: Context<'_>,
    #[description = "Subreddit to fetch from"] subreddit: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let url = format!("https://api.pullpush.io/reddit/search/submission/?subreddit={}&size=50", subreddit);
    let client = reqwest::Client::builder()
        .user_agent("archbot/0.1.0")
        .build()?;
    let response = client.get(&url).send().await?;
    if response.status().is_success() {
        let json: serde_json::Value = response.json().await?;
        if let Some(posts) = json["data"].as_array() {
            let image_posts: Vec<&serde_json::Value> = posts.iter()
                .filter(|p| {
                    if let Some(url) = p["url"].as_str() {
                        url.ends_with(".jpg") || url.ends_with(".png") || url.ends_with(".gif") || url.ends_with(".jpeg")
                    } else {
                        false
                    }
                })
                .collect();
            if !image_posts.is_empty() {
                let post_data = {
                    let mut rng = rand::rng();
                    use rand::seq::IndexedRandom; 
                    image_posts.choose(&mut rng).map(|post| {
                        (
                            post["url"].as_str().unwrap_or("").to_string(),
                            post["title"].as_str().unwrap_or("Random image").to_string(),
                            post["permalink"].as_str().unwrap_or("").to_string(),
                        )
                    })
                };
                if let Some((image_url, title, permalink)) = post_data {
                    let post_link = if permalink.is_empty() {
                        image_url.clone()
                    } else {
                        format!("https://www.reddit.com{}", permalink)
                    };
                    let embed = serenity::CreateEmbed::new()
                        .title(title)
                        .url(post_link)
                        .image(image_url)
                        .color(serenity::Colour::BLURPLE);
                    ctx.send(poise::CreateReply::default().embed(embed)).await?;
                    return Ok(());
                }
            }
            ctx.say("Could not find any direct image links in the recent posts of that subreddit.").await?;
        } else {
            ctx.say("Failed to parse the data from Pullpush. The subreddit might be empty or invalid.").await?;
        }
    } else {
        ctx.say(format!("Failed to fetch from Pullpush API. Status: {}", response.status())).await?;
    }
    Ok(())
}

#[poise::command(
    slash_command, 
    prefix_command, 
    category = "Fun",
    nsfw_only
)]
pub async fn tumblr(
    ctx: Context<'_>,
    #[description = "Tumblr blog to fetch from (e.g., 'gyzmoify' or 'gyzmoify.tumblr.com')"] blog: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let api_key = match env::var("TUMBLR_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            ctx.say("❌ The bot owner hasn't configured the Tumblr API key yet!").await?;
            return Ok(());
        }
    };
    let blog_identifier = if blog.contains(".tumblr.com") {
        blog.clone()
    } else {
        format!("{}.tumblr.com", blog)
    };
    let url = format!("https://api.tumblr.com/v2/blog/{}/posts/photo?api_key={}", blog_identifier, api_key);
    let response = reqwest::get(&url).await?;
    if response.status().is_success() {
        let json: Value = response.json().await?;
        if let Some(posts) = json["response"]["posts"].as_array() {
            let photo_posts: Vec<&Value> = posts.iter()
                .filter(|p| p["type"] == "photo")
                .filter(|p| {
                    if let Some(url) = p["photos"][0]["original_size"]["url"].as_str() {
                        !url.contains("removed")
                    } else {
                        false
                    }
                })
                .collect();
            if !photo_posts.is_empty() {
                let image_url = {
                    let mut rng = rand::rng();
                    use rand::seq::IndexedRandom;
                    photo_posts.choose(&mut rng)
                        .and_then(|post| post["photos"][0]["original_size"]["url"].as_str())
                        .map(|url| url.to_string())
                };
                if let Some(url) = image_url {
                    match reqwest::get(&url).await {
                        Ok(img_response) => {
                            if let Ok(bytes) = img_response.bytes().await {
                                let filename = url.split('/').last().unwrap_or("image.png").to_string();
                                let attachment = serenity::CreateAttachment::bytes(bytes.to_vec(), &filename);
                                let embed = serenity::CreateEmbed::new()
                                    .title(format!("Random post from {}", blog_identifier))
                                    .url(format!("https://{}", blog_identifier))
                                    .image(format!("attachment://{}", filename))
                                    .color(serenity::Colour::DARK_BLUE);
                                ctx.send(poise::CreateReply::default()
                                    .attachment(attachment)
                                    .embed(embed)
                                ).await?;
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            println!("Failed to download Tumblr image: {}", e);
                        }
                    }
                    ctx.say(format!("Found a post, but failed to fetch the image data from `{}`.", blog_identifier)).await?;
                    return Ok(());
                }
            }
            ctx.say(format!("Couldn't find any valid image posts on the blog `{}`.", blog_identifier)).await?;
        } else {
            ctx.say(format!("Couldn't parse posts from the blog `{}`.", blog_identifier)).await?;
        }
    } else {
        ctx.say(format!("Tumblr API Error: {} (Make sure the blog exists and is public)", response.status())).await?;
    }
    Ok(())
}
