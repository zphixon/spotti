#!/usr/bin/env python3

import requests
import discord
import asyncio
import time
import os
from os import path
import signal
import functools
import toml
import sys

config_file = sys.argv[1] or 'spotti-downbot.toml'
config = toml.load(open(config_file))

key = config['key']
status = config['status_file']
pidfile = config['pid_file']
me = config['me']
sent = False

intents = discord.Intents.default()
intents.members = True
intents.guilds = True
intents.messages = True
intents.message_content = True

bot = discord.Client(intents=intents)

@bot.event
async def on_message(m):
    if m.content == "!uptime":
        spotti_text = 'spotti did not respond'
        size_text = 'size did not respond'

        try:
            r = requests.get(config['spotti_uptime'])
            spotti_text = r.text
        except:
            pass

        try:
            r2 = requests.get(config['size_uptime'])
            size_text = r2.text
        except:
            pass

        await m.channel.send('spotti: ' + spotti_text + '\nsize: ' + size_text)

async def send_message():
    global status
    print('wheho')
    user = await bot.fetch_user(me)
    if path.exists(status):
        with open(status) as f:
            content = f.read()
            await user.send('spotti is dead: ' + content.strip())

@bot.event
async def on_ready():
    global status
    global sent
    print(f'{bot.user.name}: {bot.user.id}')
    bot.loop.add_signal_handler(signal.SIGUSR1, lambda: asyncio.ensure_future(send_message()))
    pid = os.getpid()
    file = open(pidfile, 'w')
    file.write(str(pid))

bot.run(key)

