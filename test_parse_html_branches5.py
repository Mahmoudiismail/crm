from bs4 import BeautifulSoup

html_content = open("CRM Updated open TKTs.htm", encoding="windows-1252", errors="ignore").read()
soup = BeautifulSoup(html_content, 'html.parser')
for div in soup.find_all('div'):
    style = div.get('style', '').replace(' ', '')
    if 'color:#44546a' in style.lower():
        print(div.text.strip())
