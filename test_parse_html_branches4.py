from bs4 import BeautifulSoup

html_content = open("CRM Updated open TKTs.htm", encoding="windows-1252", errors="ignore").read()
soup = BeautifulSoup(html_content, 'html.parser')
for b in soup.find_all('b'):
    text = b.text.strip()
    if text and text not in ["Row Labels", "OUL", "closed", "open", "% of closed", "% of open", "Grand Total"]:
        print(text)
