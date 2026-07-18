import re
from bs4 import BeautifulSoup
html_content = open("CRM Updated open TKTs.htm", encoding="windows-1252", errors="ignore").read()
soup = BeautifulSoup(html_content, 'html.parser')
for div in soup.find_all('div'):
    if 'font-weight:bold' in div.get('style', '').replace(' ', '') or 'color:#44546A' in div.get('style', '').replace(' ', ''):
        print(div.text.strip())
