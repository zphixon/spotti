#!/bin/env python3

import time
import sqlite3
import spotipy
import secret
from spotipy.oauth2 import SpotifyClientCredentials

con = sqlite3.connect('/home/zack/spotti/recents.db')
cur = con.cursor()

auth = SpotifyClientCredentials(client_id=secret.client_id, client_secret=secret.client_secret)
spotify = spotipy.Spotify(client_credentials_manager=auth)

cur.execute('select name, artist, date from songs where id is null')
rows = cur.fetchall()

for name, artist, date in rows:
    try:
        query = name + ', ' + artist
        print(date + ': ' + query + ' -> ', end='')
        id = spotify.search(query)['tracks']['items'][0]['id']
        print(id)
        cur.execute('update songs set id = ? where name = ? and artist = ?', [id, name, artist])
        con.commit()
    except IndexError:
        print('unavailable')

con.close()
