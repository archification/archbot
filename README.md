# ArchBot - The HiveMind Bot

A feature-rich Discord bot with ticket system, moderation tools, announcements, and clustering capability for high availability.

## Features

- **Clustering**: Multiple bot instances with automatic leader election
- **Ticket System**: Create and manage support tickets
- **Moderation**: Kick, ban, and manage members
- **Announcements**: Send server-wide announcements
- **Logging**: Comprehensive logging for various events
- **Configuration**: Flexible per-guild configuration
- **Stats**: Stat tracking with configurable stats
- **Countdown**: Countdowns with configurable endings
- **Tumblr**: Uploads random image from specified tumblr blog (nsfw channels only)

## Dev Setup

## Dev Setup

1. Clone this repository
2. Ensure you have the **nightly** Rust toolchain installed (the project uses nightly compiler flags for optimization).
3. (Optional) Create a `cluster.toml` file to manually define your instance ID and priority. If skipped, the bot will auto-generate a random instance ID on startup. The `config.toml` file will be created automatically.
4. Set up the required environment variables. You can export these or pass them as CLI arguments:
   * `DISCORD_TOKEN` (or run with `--dauth <token>`)
   * `COORDINATION_CHANNEL_ID` (or run with `--coordination <id>`)
   * `TUMBLR_API_KEY` (Optional: required only for the `/tumblr` command)
5. Run with `cargo run`

## Release Setup

1. Download the pre-compiled release binary.
2. (Optional) Create your `cluster.toml` in the same directory to manually configure the instance's clustering identity.
3. Provide the necessary variables in your startup script (e.g., `run.sh`):

   ```bash
   #!/bin/bash
   export DISCORD_TOKEN="your_discord_bot_token"
   export COORDINATION_CHANNEL_ID="your_channel_id"
   export TUMBLR_API_KEY="your_api_key" # Optional
   
   ./archbot

## Running as a Service (Linux)

To keep ArchBot running in the background, you can create a `systemd` service. 
Create a file at `/etc/systemd/system/archbot.service`:

```ini
[Unit]
Description=ArchBot Discord Daemon
After=network.target

[Service]
Type=simple
User=archbot
WorkingDirectory=/opt/archbot
ExecStart=/opt/archbot/archbot --dauth YOUR_TOKEN --coordination YOUR_CHANNEL_ID
Restart=always
RestartSec=5

# Optional: Load environment variables from a file
# EnvironmentFile=/opt/archbot/.env

[Install]
WantedBy=multi-user.target
```
Enable and start the bot with:
`sudo systemctl enable --now archbot`

## Configuration

### Cluster Configuration (`cluster.toml`)
```toml
[cluster]
# Unique identifier for this instance
instance_id = "unique-instance-name"
# Leadership priority (lower = more likely to be leader)
priority = 1
```

### Bot Confuration
The bot automatically creates an empty config.toml file if it doesn't exist.
Example structure:
```toml
[guild_id]
announcement_channel = 1234567890
custom_stats = [
    "one",
    "two",
    "three",
]
logging_channel = 1234567890         # Default logging channel
member_log_channel = 1234567890      # Member join/leave logs
ticket_category = 1234567890         # Category for ticket channels
ticket_log_channel = 1234567890      # Ticket activity logs
mod_log_channel = 1234567890         # Moderation action logs
boot_quit_channel = 1234567890       # Bot startup/shutdown notifications
ticket_roles = [1234567890]          # Roles with ticket access
ticket_exempt_role = 1234567890      # Role exempt from seeing ticket message

[guild_id.countdown_endings]
asdf = "the asdf is now foobar"
guacamole = "the avocado may now be mashed"
```
Running the `/help config` command will show all available subcommands for configuration.

## Commands

### General Commands
* `help [command]` - Shows help menu
* `vote <choice>` - Vote for something
* `getvotes [choice]` - Show vote counts
* `stat [stat_name] <amount>` - Show vote counts
* `viewstat [stat_name] [user]` - Show vote counts
* `diceroll [dice]` - Roll dice in XdY format with optional math (e.g., 2d6+5)
* `countdown [start] [difficulty] [ending]` - Start a probalistic countdown
* `stat <stat_name> [amount]` - Add to a personal tracked stat
* `viewstat [stat_name] [user]` - View personal or server-wide user stats
* `tumblr <blog>` - Fetch a random photo from a specific Tumblr blog (NSFW channels only)

### Ticket Commands
* `ticket [issue]` - Create a new support ticket
* `closeticket [reason]` - Close the current ticket (admin-only)

### Moderation Commands
* `kick <user> [reason]` - Kick a user (admin-only)
* `ban <user> [reason] [delete_message_days]` - Ban a user (admin-only)
* `announce <message>` - Make an announcement (admin-only)

### Configuration Commands (admin-only, leader-only)
* `config` - Show configuration commands
* `view` - Show current configuration
    * Types: default, boot, member, ticket, mod, message, announcement
* `set_logging_channel [type] [channel]` - Sets logs of a certain type to a specific channel
* `ticket_category <category>` - Set ticket category
* `add_ticket_role <role>` - Add role to ticket access
* `remove_ticket_role <role>` - Remove role from ticket access
* `ticket_message <text_file>` - Set ticket message template (upload .txt file)
* `ticket_exempt_role <role>` - Set role exempt from ticket message
* `remove_ticket_exempt_role` - Remove ticket exempt role

### Owner Commands
* `quit` - Shutdown all bot instances
* `quit [instance id]` Shutdown specific bot instance
* `writeconfig` - Force save config to disk (owner-only)
* `register` - Force command registration sync

## Clustering
Multiple instances with automatic leader election:
* Instances communicate via a dedicated Discord channel
* Heartbeats are sent every 10 seconds
* Leader timeout is 60 seconds
* Highest priority instance becomes leader (with oldest instance as tiebreaker)
* Only the leader executes commands

## Event Logging
Events are logged to configured channels:
* Bot startup/shutdown
* Member joins/leaves
* Ticket creation/closing
* Moderation actions (kicks/bans)
* Announcements
* Message edits and deletions

## Ticket System Features
* Customizable ticket message templates
* Role-based access control
* Exempt roles that don't see the ticket message
* Automatic channel creation with proper permissions

## Requirements
* Rust 1.70+
* Discord bot token with these intents:
    * Guilds
    * Guild Members
    * Guild Messages
    * Message Content
