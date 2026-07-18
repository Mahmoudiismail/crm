from bs4 import BeautifulSoup
html_content = open("CRM Updated open TKTs.htm", encoding="windows-1252", errors="ignore").read()
soup = BeautifulSoup(html_content, 'html.parser')
tables = soup.find_all('table')
if len(tables) > 0:
    for tr in tables[0].find_all('tr')[:5]:
        print(tr)
