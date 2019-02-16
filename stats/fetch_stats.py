import pickle
import json
import base64
from bs4 import BeautifulSoup
import email
import os.path
from googleapiclient.discovery import build
from google_auth_oauthlib.flow import InstalledAppFlow
from google.auth.transport.requests import Request

# If modifying these scopes, delete the file token.pickle.
SCOPES = ['https://www.googleapis.com/auth/gmail.readonly']


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

    data = []

    for i, message in enumerate(messages[:10]):
        message = service.users().messages().get(
            userId='me',
            id=message['id'],
            format='raw',
        ).execute()

        msg_str = base64.urlsafe_b64decode(message['raw'].encode('ASCII'))
        mime_msg = email.message_from_string(str(msg_str))

        soup = BeautifulSoup(str(mime_msg), 'html.parser')
        rows = soup.find_all("pre")

        for row in rows:
            try:
                row = row.contents
                stats = json.loads(row[0])
                data.append(stats)
            except:
                pass

    return data[::-1]


if __name__ == '__main__':
    main()
