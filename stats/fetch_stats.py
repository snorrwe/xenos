import pickle
import json
import base64
from bs4 import BeautifulSoup
import email
import os.path
from googleapiclient.discovery import build
from google_auth_oauthlib.flow import InstalledAppFlow
from google.auth.transport.requests import Request
import psycopg2

SCOPES = ['https://www.googleapis.com/auth/gmail.readonly']

PG_HOST = "localhost"
PG_PORT = "5432"
PG_DB = "screeps"
PG_USER = "screeps"
PG_PW = "screepsbois"


def load_data(messages, service):

    data = []

    for i, message in enumerate(messages):
        message = service.users().messages().get(
            userId='me',
            id=message['id'],
            format='raw',
        ).execute()

        msg_str = base64.urlsafe_b64decode(message['raw'].encode('ASCII'))
        mime_msg = email.message_from_string(str(msg_str))

        soup = BeautifulSoup(str(mime_msg), 'html.parser')
        rows = soup.find_all("pre")

        for row in rows[::-1]:
            try:
                row = row.contents
                stats = json.loads(row[0])
                data.append(stats)
            except Exception:
                pass

    connection = psycopg2.connect(
        user=PG_USER,
        password=PG_PW,
        host=PG_HOST,
        port=PG_PORT,
        database=PG_DB)

    cursor = connection.cursor()

    table_name = "screeps_data"

    try:
        cursor.execute(f"""
            CREATE TABLE IF NOT EXISTS {table_name}
            (
                time INTEGER PRIMARY KEY
                , bucket INTEGER NOT NULL
                , cpu INTEGER NOT NULL
                , population INTEGER NOT NULL

            );
            """)
    except Exception as e:
        print('!!', e)

    command = f"""
    INSERT INTO {table_name} (time, bucket, cpu, population)
    VALUES (%(time)s, %(bucket)s, %(cpu)s, %(population)s)
"""

    for row in data:
        try:
            cursor.execute(command, row)
        except Exception:
            pass
    connection.commit()

    return data[::-1]


def main():
    """Shows basic usage of the Gmail API.
    Lists the user's Gmail labels.
    """
    creds = None
    # The file token.pickle stores the user's access and refresh tokens, and is
    # created automatically when the authorization flow completes for the first
    # time.
    if os.path.exists('token.pickle'):
        with open('token.pickle', 'rb') as token:
            creds = pickle.load(token)
    # If there are no (valid) credentials available, let the user log in.
    if not creds or not creds.valid:
        if creds and creds.expired and creds.refresh_token:
            creds.refresh(Request())
        else:
            flow = InstalledAppFlow.from_client_secrets_file(
                'credentials.json', SCOPES)
            creds = flow.run_local_server()
        # Save the credentials for the next run
        with open('token.pickle', 'wb') as token:
            pickle.dump(creds, token)

    service = build('gmail', 'v1', credentials=creds)

    # Call the Gmail API
    result = service.users().messages().list(
        # TODO: find label by name "Screeps"
        userId='me',
        labelIds="Label_5").execute()
    messages = result["messages"]

    result = load_data(messages, service)
    return result


if __name__ == '__main__':
    main()
