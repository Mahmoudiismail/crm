from bs4 import BeautifulSoup

html = open("CRM Updated open TKTs.htm", encoding="windows-1252", errors="ignore").read()
soup = BeautifulSoup(html, "html.parser")
tables = soup.find_all("table")
print(tables[0].find_all("tr")[0])
