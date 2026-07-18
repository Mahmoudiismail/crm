from bs4 import BeautifulSoup

html_content = open("CRM Updated open TKTs.htm", encoding="windows-1252", errors="ignore").read()
soup = BeautifulSoup(html_content, 'html.parser')
for p in soup.find_all('p'):
    if 'font-weight:bold' in p.get('style', '').replace(' ', '') or 'color:#44546A' in p.get('style', '').replace(' ', ''):
        print(p.text.strip())
