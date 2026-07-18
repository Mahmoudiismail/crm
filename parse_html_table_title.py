from bs4 import BeautifulSoup

html_content = open("CRM Updated open TKTs.htm", encoding="windows-1252", errors="ignore").read()
soup = BeautifulSoup(html_content, 'html.parser')
for p in soup.find_all('p'):
    text = p.text.strip()
    if "(" in text and ")" in text and "2026" in text:
        print(text)
