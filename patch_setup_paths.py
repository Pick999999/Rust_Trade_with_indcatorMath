import os

files_to_patch = [
    'src/main.rs',
    'src/full_analysis_Ver2.rs',
    'src/full_analysis.rs',
    'src/deriv.rs'
]

for file in files_to_patch:
    if os.path.exists(file):
        with open(file, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Replace
        content = content.replace('\"setup.json\"', '\"setup/setup.json\"')
        content = content.replace('\"thereshold.json\"', '\"setup/thereshold.json\"')
        
        # Fix the format strings
        content = content.replace('\"{}/setup/setup.json\"', '\"{}/setup.json\"')
        content = content.replace('\"{}/setup/thereshold.json\"', '\"{}/thereshold.json\"')
        
        with open(file, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f'Patched {file}')
