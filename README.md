# OmniCore Discord Bot - Fixed Public Output Version

This is the fixed version of OmniCore.

## What changed in this version

1. **Server owner/Admin permission fix**
   - Commands now always allow the server owner.
   - Commands also allow users with Administrator.
   - Slash commands no longer use Discord-side `default_member_permissions`, so Discord's command permissions UI should not block the owner by mistake.

2. **Public bot messages**
   - The visible output is sent with `channel.send()`.
   - That means the message appears as a normal bot message in the server channel.
   - The slash command still gets a tiny private acknowledgement saying `Posted publicly` because Discord requires the interaction to be answered.

3. **Role hierarchy warnings**
   - If `/roleadd` or `/roleremove` says the bot cannot add/remove a role, go to:
     `Server Settings -> Roles`
   - Drag the OmniCore bot role **above** the role you want it to manage.
   - Discord blocks bots from managing roles higher than their highest role, even if the bot has Administrator.

4. **Ollama mention replies**
   - Mention the bot in a channel with a prompt and it will ask Ollama for a response.
   - The Ollama server URL and model are configured in `.env`.

## Folder setup

Extract the ZIP so your folder looks like:

```text
C:\OmniCore_DiscordBot
│  main.py
│  requirements.txt
│  README.md
│  .env.example
│
├─ cogs
│  │  moderation.py
│  │  utils.py
│  │  __init__.py
│
└─ data
```

Keep your existing `.env` file if you already made one.

## .env file

Rename `.env.example` to `.env` and put your bot token in it:

```env
DISCORD_TOKEN=YOUR_REAL_BOT_TOKEN_HERE
OLLAMA_BASE_URL=http://127.0.0.1:11434
OLLAMA_MODEL=
```

If Ollama is running on another machine, set `OLLAMA_BASE_URL` to that machine,
for example `http://192.168.1.50:11434`. Leave `OLLAMA_MODEL` blank to use the
first installed model, or set it to a model from `ollama list`, such as
`llama3.2:latest`.

Do not share your token.

## Install dependencies

Open Command Prompt:

```bat
cd C:\OmniCore_DiscordBot
pip install -r requirements.txt
```

## Enable required Discord intent

Go to the Discord Developer Portal -> your app -> Bot -> Privileged Gateway Intents.

Turn on:

```text
Server Members Intent
Message Content Intent
```

Then click **Save Changes**.

## Invite permissions

When inviting the bot, include these scopes:

```text
bot
applications.commands
```

Recommended bot permissions:

```text
View Channels
Send Messages
Embed Links
Read Message History
Manage Roles
Manage Channels
Manage Messages
Kick Members
Ban Members
Moderate Members
Manage Nicknames
```

For testing, Administrator is okay, but for a real server only give the permissions you need.

## Run the bot

```bat
cd C:\OmniCore_DiscordBot
python -B main.py
```

The `-B` option tells Python not to use cached `.pyc` files while testing.

## Important after replacing files

If you copied this over an older version, delete old cache files:

```bat
cd C:\OmniCore_DiscordBot
rmdir /s /q __pycache__
rmdir /s /q cogs\__pycache__
python -B main.py
```

It is okay if Windows says one of those folders does not exist.

## Commands

- `/mute user reason`
- `/unmute user`
- `/timeout user duration reason`
- `/untimeout user`
- `/ban user reason`
- `/unban user_id`
- `/kick user reason`
- `/warn user reason`
- `/warnings user`
- `/clearwarnings user`
- `/purge amount`
- `/lock`
- `/unlock`
- `/slowmode delay`
- `/nick user nickname`
- `/roleadd user role`
- `/roleremove user role`
- `/userinfo user`
- `/serverinfo`
- `/avatar user`
- `/ping`
- `/help`

## Ollama mentions

After the bot is running, mention it in any channel it can read and send messages
in:

```text
@OmniCore explain what this server is for
```

The bot strips its own mention from the message, sends the rest to Ollama, and
replies in the same channel.

## Why does slash command still show a tiny private acknowledgement?

Discord requires slash commands to receive a response, or it shows "interaction failed." This version sends the actual result publicly as a normal bot message, then privately acknowledges the slash command with `Posted publicly`.

The normal visible message should show from the OmniCore bot account in the channel.
