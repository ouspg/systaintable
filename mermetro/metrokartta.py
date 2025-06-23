import json
import time
import sys
from pytz import timezone
from datetime import datetime, timedelta
from flask import Flask, render_template_string, jsonify
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler

app = Flask(__name__)

current_metromap = ""

def convert_to_finnish_time(timestamp_str):
    """
    Muuntaa UTC timestampin Suomen kesä- tai talviaikaan
    Palauttaa onnistuessaan merkkijonon muotoa 22.11.2022 22:22
    """
    if timestamp_str == 'N/A':
        return 'N/A'
    
    try:
        utc_dt = datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
        finnish_tz = timezone('Europe/Helsinki')
        finnish_dt = utc_dt.astimezone(finnish_tz)
        return finnish_dt.strftime('%d.%m.%Y %H:%M')
        
    except Exception as e:
        print(f"Virhe aikammuunnoksessa: {e}")
        return timestamp_str
        
def parse_identities(entry):
    """
    Funktio muuntaa JSON-entry tunnisteet mermaidin metrokartta koodiksi
    Palauttaa listan tunnisteista jotka voidaan lisätä metrokarttaan
    """
    identities = []
    
    if entry['type'] == 'IP-osoite':
        identities.append(f'IPv4_{entry["value"].replace(".", "_")}([IP-Address<br/>{entry["value"]}])')
    elif entry['type'] == 'MAC-osoite':
        identities.append(f'MAC_{entry["value"].replace(":", "_")}([MAC-Address<br/>{entry["value"]}])')
    elif entry['type'] == 'Käyttäjä':
        identities.append(f'User_{entry["value"].replace(" ", "_")}([User<br/>{entry["value"]}])')
    elif entry['type'] == 'Email':
        identities.append(f'Email_{entry["value"].replace("@", "_AT_").replace(".", "_")}([Email<br/>{entry["value"]}])')
    elif entry['type'] == 'Hostname':
        identities.append(f'Hostname_{entry["value"].replace(".", "_")}([Hostname<br/>{entry["value"]}])')
    elif entry['type'] == 'asd':
        identities.append(f'ASD_{entry["value"].replace(" ", "_")}([ASD<br/>{entry["value"]}])')
    return identities

def group_by_person(connections):
    """
    Luo lokien entryistä henkilöryhmiä
    Palauttaa listana henkilöryhmät, joissa jokainen ryhmä on joukko tunnisteita
    """
    person_groups = []
    processed = set()
    
    for a, b in connections:
        if a in processed and b in processed:
            continue
            
        # Etsitään onko jompikumpi jo jossain ryhmässä
        found_group = None
        for group in person_groups:
            if a in group or b in group:
                found_group = group
                break
        
        if found_group:
            found_group.add(a)
            found_group.add(b)
        else:
            person_groups.append({a, b})
        
        processed.add(a)
        processed.add(b)
    
    return person_groups

def generate_metromap_content(all_nodes, connections):
    """
    Funktio luo metrokartta-sisällön merkkijonona
    Palauttaa merkkijonon joka sisältää metrokartalle kaavion
    """
    content = "flowchart RL\n"
    content += "    LOKITIEDOSTO[(Lokitiedosto)]\n\n"
    
    person_groups = group_by_person(connections)
    processed_nodes = set()
    
    colors = [
        ("#4CAF50", "#2E7D32"),
        ("#2196F3", "#1565C0"),
        ("#9C27B0", "#6A1B9A"),
        ("#FF9800", "#E65100"),
        ("#F44336", "#C62828"),
        ("#795548", "#3E2723"),
        ("#607D8B", "#263238"),
        ("#E91E63", "#AD1457")
    ]
    
    all_connections = []
    all_styles = []
    
    # Käsittellään jokainen henkilöryhmä
    for group_num, group in enumerate(person_groups):
        group_list = list(group)
        color_index = group_num % len(colors)
        fill_color, stroke_color = colors[color_index]
        
        if len(group_list) == 1:
            node = group_list[0]
            node_id = node.split('(')[0]
            
            content += f"    {node}\n"
            all_connections.append(f"    LOKITIEDOSTO --- {node_id}")
            
            all_styles.append(f"    style {node_id} fill:{fill_color},stroke:{stroke_color},color:#fff,stroke-width:3px")
            
            connection_index = len(all_connections) - 1
            all_styles.append(f"    linkStyle {connection_index} stroke:{fill_color},stroke-width:5px")
            
            processed_nodes.add(node)
            
        else:
            # Järjestys: User > Email > IP > MAC > Hostname > etc
            main_node = None
            
            # 1. Käyttäjä
            for node in group_list:
                if "User_" in node:
                    main_node = node
                    break
            
            # 2. Email
            if not main_node:
                for node in group_list:
                    if "Email_" in node:
                        main_node = node
                        break
            
            # 3. IPv4
            if not main_node:
                for node in group_list:
                    if "IPv4_" in node:
                        main_node = node
                        break
            
            # 4. MAC
            if not main_node:
                for node in group_list:
                    if "MAC_" in node:
                        main_node = node
                        break
            
            # 5. Hostname
            if not main_node:
                for node in group_list:
                    if "Hostname_" in node:
                        main_node = node
                        break
            
            # 6. Ensimmäinen jäljellä oleva
            if not main_node:
                main_node = group_list[0]
            
            sub_nodes = [node for node in group_list if node != main_node]

            MAX_FIRST_LEVEL = 3
            
            # Luodaan pääsolmu
            content += f"    {main_node}\n"
            main_node_id = main_node.split('(')[0]
            
            # Luodaan alisolmut
            for sub_node in sub_nodes:
                content += f"    {sub_node}\n"
            
            content += "\n"
            
            # Yhdistetään pääsolmu lokitiedostoon
            all_connections.append(f"    LOKITIEDOSTO --- {main_node_id}")
            
            if sub_nodes:
                # _ax 3 solmua yhdistyy pääsolmuun
                first_level = sub_nodes[:MAX_FIRST_LEVEL]
                remaining_nodes = sub_nodes[MAX_FIRST_LEVEL:]
                
                # Yhdistä ensimmäinen taso pääsolmuun
                for sub_node in first_level:
                    sub_node_id = sub_node.split('(')[0]
                    all_connections.append(f"    {main_node_id} --- {sub_node_id}")
                
                # Jokaisesta ensimmäisen tason solmusta max 1 jatke
                current_level = first_level
                remaining = remaining_nodes
                
                while remaining and current_level:
                    next_level = []
                    
                    # Jokaisesta nykyisen tason solmusta max 1 yhteys
                    for i, parent_node in enumerate(current_level):
                        if i < len(remaining):  # Jos on vielä yhteydettäviä solmuja
                            child_node = remaining[i]
                            parent_id = parent_node.split('(')[0]
                            child_id = child_node.split('(')[0]
                            
                            all_connections.append(f"    {parent_id} --- {child_id}")
                            next_level.append(child_node)
                    
                    # Päivitä seuraavaa kierrosta varten
                    current_level = next_level
                    remaining = remaining[len(next_level):]
            
            # Sama väri koko linjalle
            all_styles.append(f"    style {main_node_id} fill:{fill_color},stroke:{stroke_color},color:#fff,stroke-width:3px")
            for sub_node in sub_nodes:
                sub_node_id = sub_node.split('(')[0]
                all_styles.append(f"    style {sub_node_id} fill:{fill_color},stroke:{stroke_color},color:#fff,stroke-width:3px")
            
            # Link-tyylit
            total_connections = 1 + len(sub_nodes)
            connection_start = len(all_connections) - total_connections
            for i in range(total_connections):
                all_styles.append(f"    linkStyle {connection_start + i} stroke:{fill_color},stroke-width:5px")
            
            processed_nodes.update(group_list)
    
    # Yksittäiset solmut joilla ei ole yhteyksiä
    orphan_nodes = all_nodes - processed_nodes
    for i, node in enumerate(sorted(orphan_nodes)):
        color_index = (len(person_groups) + i) % len(colors)
        fill_color, stroke_color = colors[color_index]
        
        node_id = node.split('(')[0]
        content += f"    {node}\n"
        
        all_connections.append(f"    LOKITIEDOSTO --- {node_id}")
        all_styles.append(f"    style {node_id} fill:{fill_color},stroke:{stroke_color},color:#fff,stroke-width:3px")
        
        connection_index = len(all_connections) - 1
        all_styles.append(f"    linkStyle {connection_index} stroke:{fill_color},stroke-width:5px")
    
    # Lisätään yhteydet
    content += "\n    %% Yhteydet\n"
    for connection in all_connections:
        content += f"{connection}\n"
    
    # Tyylitä lokitiedosto
    content += "\n    %% Tyylit\n"
    content += "    style LOKITIEDOSTO fill:#424242,stroke:#212121,color:#fff,stroke-width:6px\n"
    content += "    classDef rootNode font-size:48px,font-weight:bold\n"
    content += "    class LOKITIEDOSTO rootNode\n"
    
    # Lisää muut tyylit
    for style in all_styles:
        content += f"{style}\n"
    
    return content

def process_json_file():
    """
    Funktio käy läpi json tiedostoa ja muodostaa valmiin Mermaid-koodin
    Koodi sijoitetaan globaalin current_metromap muuttujaan
    Funktio ei palauta mitään
    """
    global current_metromap
    
    all_nodes = set()
    connections = []
    node_timestamps = {}
    
    try:
        with open("lokitiedosto.json", "r", encoding="utf-8") as f:
            data = json.load(f)
        
        # Ryhmittellään entryt ensin riveittäin
        lines = {}
        for entry in data:
            if 'line' in entry:
                line_num = entry['line']
                if line_num not in lines:
                    lines[line_num] = []
                lines[line_num].append(entry)
        
        # Tarkistetaan lokitiedoston sisältö
        print(f"Käsitellään {len(lines)} riviä lokitiedostosta...")
        if len(lines) == 0:
            print("Lokitiedosto on tyhjä tai ei sisällä rivejä.")
            sys.exit()

        # Käydään läpi jokainen rivi ja kerätään tunnisteet
        for line_num, entries in lines.items():
            all_line_ids = []
            
            for entry in entries:
                ids = parse_identities(entry)
                all_line_ids.extend(ids)
                
                # Tallennetaan timestampit
                for node_id in ids:
                    node_key = node_id.split('(')[0]
                    timestamp = entry.get('timestamp', 'N/A')
                    node_timestamps[node_key] = timestamp
            
            if all_line_ids:
                for n in all_line_ids:
                    all_nodes.add(n)
                
                # Yhdistetään saman rivin tunnisteet
                for i in range(len(all_line_ids)):
                    for j in range(i+1, len(all_line_ids)):
                        connections.append((all_line_ids[i], all_line_ids[j]))
        
        # Lisää timestampit solmuihin
        updated_nodes = set()
        for node in all_nodes:
            node_key = node.split('(')[0]
            timestamp = node_timestamps.get(node_key, 'N/A')
            formatted_timestamp = convert_to_finnish_time(timestamp)
            
            if '([' in node and node.endswith('])'):
                parts = node.split('([')
                if len(parts) == 2:
                    node_id = parts[0]
                    content = parts[1][:-2]
                    updated_node = f"{node_id}([{content}<br/>{formatted_timestamp}])"
                    updated_nodes.add(updated_node)
                else:
                    updated_nodes.add(node)
            else:
                updated_nodes.add(node)
        
        # Muodostetaan päivitetyt yhteydet
        updated_connections = []
        for a, b in connections:
            a_key = a.split('(')[0]
            b_key = b.split('(')[0]
            
            # Etsitään vastaavat päivitetyt nodet
            updated_a = None
            updated_b = None
            for node in updated_nodes:
                if node.split('(')[0] == a_key:
                    updated_a = node
                if node.split('(')[0] == b_key:
                    updated_b = node
            
            if updated_a and updated_b:
                updated_connections.append((updated_a, updated_b))
        
        current_metromap = generate_metromap_content(updated_nodes, updated_connections)
        print(f"Metrokartta päivitetty: {datetime.now().strftime('%H:%M:%S')}")
        
    except Exception as e:
        print(f"Virhe JSON-tiedoston käsittelyssä: {e}")
    #Funktio päättyy tähän

class JSONFileHandler(FileSystemEventHandler):
    """
    Tiedoston seurantaluokka joka reagoi JSON-tiedoston muutoksiin
    Kun lokitiedosto muuttuu, kutsutaan process_json_file funktiota
    """
    def on_modified(self, event):
        if event.src_path.endswith("lokitiedosto.json"):
            print("JSON-tiedosto muuttui, päivitetään metrokartta...")
            time.sleep(0.2)
            process_json_file()

@app.route('/')
def index():
    """Pääsivu joka näyttää metrokartan"""
    html_template = '''
<!DOCTYPE html>
<html>
<head>
    <title>Yhteenveto</title>
    <script type="module">
        import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';
        mermaid.initialize({ startOnLoad: true });
        
        setInterval(async function() {
            try {
                const response = await fetch('/api/metromap');
                const data = await response.json();
                
                if (data.metromap !== window.lastMetromap) {
                    window.lastMetromap = data.metromap;
                    document.getElementById('metromap-container').innerHTML = 
                        '<div class="mermaid">' + data.metromap + '</div>';
                    mermaid.init();

                    document.getElementById('timestamp').textContent = data.timestamp;
                }
            } catch (error) {
                console.error('Virhe päivityksessä:', error);
            }
        }, 2000);
    </script>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }
        .status { color: #4CAF50; font-weight: bold; }
        .mermaid { text-align: center; background-color: white; padding: 20px; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        h1 { color: #333; text-align: center; }
        .info { background-color: white; padding: 15px; border-radius: 5px; margin-bottom: 20px; }
    </style>
</head>
<body>
    <h1>Yhteenveto lokitiedostosta</h1>
    <div class="info">
        <p class="status">Päivittyy automaattisesti kun JSON-tiedosto muuttuu</p>
        <p>Viimeksi päivitetty: <span id="timestamp">{{ timestamp }}</span></p>
    </div>
    
    <div id="metromap-container">
        <div class="mermaid">{{ metromap|safe }}</div>
    </div>
</body>
</html>
    '''
    
    return render_template_string(html_template, 
                                  metromap=current_metromap,
                                  timestamp=datetime.now().strftime('%H:%M:%S'))

@app.route('/api/metromap')
def api_metromap():
    """API josta haetaan nykyinen metrokartta"""
    return jsonify({
        'metromap': current_metromap,
        'timestamp': datetime.now().strftime('%H:%M:%S')
    })

def create_html_file(metromap_content):
    """
    Funktio luo aktiiviseen hakemistoon HTML-tiedoston,
    metrokarttaa voidaan katsoa myös staattisena tiedostona
    Funtkio ei palauta mitään
    """
    html_content = f"""<!DOCTYPE html>
<html>
<head>
    <title>Metrokartta</title>
    <script type="module">
        import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';
        mermaid.initialize({{ startOnLoad: true }});
    </script>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }}
        .mermaid {{ text-align: center; background-color: white; padding: 20px; border-radius: 10px; }}
    </style>
</head>
<body>
    <h1>Metrokartta</h1>
    <div class="mermaid">
{metromap_content}
    </div>
</body>
</html>"""
    
    with open('metrokartta.html', 'w', encoding='utf-8') as f:
        f.write(html_content)

def start_file_watcher():
    """JSON-tiedoston seuranta käynnistetään taustalle"""
    event_handler = JSONFileHandler()
    observer = Observer()
    observer.schedule(event_handler, path='.', recursive=False)
    observer.start()
    print("Tiedostoa luetaan taustalla...")
    return observer

def main():
    print("Käynnistetään metrokartta-palvelin")
    print(f"Aika: {datetime.now().strftime('%d.%m.%Y %H:%M:%S')}")
    process_json_file()
    observer = start_file_watcher()

    metromap_content_for_files = current_metromap
    with open('metrokartta_koodi.txt', 'w', encoding='utf-8') as f:
        f.write(metromap_content_for_files)
    create_html_file(metromap_content_for_files)
    print("\nLuotu kaksi tiedostoa:")
    print("   metrokartta_koodi.txt  -> sisältää mermaid koodin")
    print("   metrokartta.html  -> sisältää mermaid diagrammin")
    
    print("\nPääsy:")
    print("   Live-sivu: http://localhost:5000")
    print("   Staattinen: metrokartta.html")
    print(" ")
    
    try:
        app.run(debug=False, host='127.0.0.1', port=5000)
    except KeyboardInterrupt:
        print("\nPalvelin pysäytetty")
    finally:
        observer.stop()
        observer.join()

if __name__ == "__main__":
    main()
