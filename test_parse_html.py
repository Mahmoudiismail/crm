from bs4 import BeautifulSoup
html_content = open("CRM Updated open TKTs.htm", encoding="windows-1252", errors="ignore").read()
soup = BeautifulSoup(html_content, 'html.parser')
tables = soup.find_all('table')
print(f"Number of tables: {len(tables)}")
for i, t in enumerate(tables):
    print(f"Table {i+1} rows: {len(t.find_all('tr'))}")
