import sys

def split_css():
    with open('static/style.css', 'r') as f:
        lines = f.readlines()

    files = {
        'base.css': [],
        'layout.css': [],
        'components.css': [],
        'cards.css': [],
        'search.css': [],
        'detail.css': [],
        'editor.css': [],
        'auth.css': [],
        'responsive.css': [],
    }

    current_file = 'base.css'
    in_media = False
    brace_count = 0
    
    for i, line in enumerate(lines):
        num = i + 1
        
        # Determine main section
        if not in_media:
            if num == 1: current_file = 'base.css'
            elif num == 93: current_file = 'layout.css'
            elif num == 123: current_file = 'components.css'
            elif num == 233: current_file = 'layout.css'
            elif num == 291: current_file = 'cards.css'
            elif num == 556: current_file = 'search.css'
            elif num == 1388: current_file = 'search.css'

        # Check for media queries
        if '@media' in line:
            in_media = True
            
        if in_media:
            files['responsive.css'].append(line)
            brace_count += line.count('{')
            brace_count -= line.count('}')
            if brace_count == 0 and '{' in line or (brace_count == 0 and '}' in line):
                in_media = False
        else:
            files[current_file].append(line)

    for name, lines in files.items():
        if lines:
            with open(f'static/css/{name}', 'w') as f:
                f.writelines(lines)
        else:
            with open(f'static/css/{name}', 'w') as f:
                f.write('/* ' + name + ' */\n')

split_css()
