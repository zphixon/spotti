#!/usr/bin/env python3

import sqlite3
import spotipy
from spotipy import SpotifyOAuth

redirect_uri = 'http://localhost:44554/spotti'
scope = 'user-library-read user-read-recently-played user-read-playback-state'
client_id = 'db455b8031ba46299ee58856f250bc02'
client_secret = 'ba3e857603cc4ac788976d198f43c23c'
response_type = 'code'

oauth = SpotifyOAuth(redirect_uri=redirect_uri, client_id=client_id, client_secret=client_secret, scope=scope)
#print(oauth.get_authorize_url())
#auth = spotipy.util.prompt_for_user_token(client_id=client_id, client_secret=client_secret, redirect_uri=redirect_uri)

#spotify = spotipy.Spotify(auth=auth)
spotify = spotipy.Spotify(client_credentials_manager=oauth)

results = spotify.current_user_recently_played(50)

conn = sqlite3.connect('recents.db')
c = conn.cursor()

c.execute('create table if not exists songs (name text, album text, artist text, date text unique)')

for track in results['items']:
    time = track['played_at']
    track = track['track']
    try:
        artists = ''
        for i, artist in enumerate(track['artists']):
            if i+1 < len(track['artists']):
                artists += artist['name'] + ', '
            else:
                artists += artist['name']

        c.execute('insert into songs values (?, ?, ?, ?)', [track['name'], track['album']['name'], artists, time])
    except Exception as e:
        pass

conn.commit()
conn.close()
