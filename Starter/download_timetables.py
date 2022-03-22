"""

Downloads rail time tables from open rail data group

Explained here: https://wiki.openraildata.com/DTD#Advice_on_downloading_data
Specification for timetable files here; https://www.raildeliverygroup.com/files/Publications/services/rsp/RSPS5046-01-00_Timetable_Information_Data_Feed_InterfaceSpecification.pdf

NOTE: As per the wiki, you should also download and apply the "updates" files... This currently doesnt do that.

"""

import requests


CREDENTIAL_FILE = "credentials.txt"

AUTH_ENDPOINT = "https://opendata.nationalrail.co.uk/authenticate"

TIMETABLE_ENDPOINT = "https://opendata.nationalrail.co.uk/api/staticfeeds/3.0/timetable"


def main():
    with open(CREDENTIAL_FILE, "r") as f:
        username, password = [s.strip() for s in f.read().split("\n")[:2]]
    
    auth_data = {
        "username": username,
        "password": password
    }
    auth_rsp = requests.post(AUTH_ENDPOINT, data=auth_data)
    auth_token = auth_rsp.json()["token"]

    headers = {"X-Auth-Token": auth_token}

    tt_rsp = requests.get(TIMETABLE_ENDPOINT, headers=headers)
    with open("out.zip", "wb") as f:
        f.write(tt_rsp.content)
    
    # import code
    # code.interact(local=locals())


if __name__ == "__main__":
    main()
