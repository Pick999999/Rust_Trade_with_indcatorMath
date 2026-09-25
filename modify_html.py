import re

with open("D:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/public/index.html", "r", encoding="utf-8") as f:
    content = f.read()

# 1. Replace the header and <main class="container"> with the new layout
header_pattern = r'<!-- ─── HEADER ─────────────────────────────────────────── -->\s*<header class="header">.*?</header>\s*<!-- ─── MAIN ────────────────────────────────────────────── -->\s*<main class="container">'

new_layout = """<!-- ─── APP LAYOUT ─────────────────────────────────────── -->
    <div class="app-layout" style="display: flex; height: 100vh; overflow: hidden;">
        
        <!-- ─── LEFT PANEL ─────────────────────────────────────────── -->
        <aside class="left-panel" style="width: 250px; background: var(--header-bg); color: var(--header-text); display: flex; flex-direction: column; border-right: 1px solid rgba(255,255,255,0.08); z-index: 100; flex-shrink: 0; overflow-y: auto;">
            <div style="padding: 20px; display: flex; align-items: center; gap: 12px; border-bottom: 1px solid rgba(255,255,255,0.08);">
                <img src="images/mylogo.png" alt="PK Logo" style="width: 40px; height: 40px; border-radius: 8px; object-fit: cover;">
                <h1 class="header-title" style="font-size: 16px; font-weight: 600; color: var(--header-text); margin: 0;">PK Deriv Trade</h1>
            </div>

            <div style="padding: 20px; display: flex; flex-direction: column; gap: 12px;">
                <button class="settings-btn" id="openTradeSettingsBtn" style="width: 100%; justify-content: flex-start; padding: 10px 14px; font-size: 14px; background: rgba(255,255,255,0.1); border-color: rgba(255,255,255,0.3);">
                    <svg viewBox="0 0 20 20" fill="currentColor" style="width:16px; height:16px; margin-right:6px;"><path fill-rule="evenodd" d="M11.49 3.17c-.38-1.56-2.6-1.56-2.98 0a1.532 1.532 0 01-2.286.948c-1.372-.836-2.942.734-2.106 2.106.54.886.061 2.042-.947 2.287-1.561.379-1.561 2.6 0 2.978a1.532 1.532 0 01.947 2.287c-.836 1.372.734 2.942 2.106 2.106a1.532 1.532 0 012.287.947c.379 1.561 2.6 1.561 2.978 0a1.533 1.533 0 012.287-.947c1.372.836 2.942-.734 2.106-2.106a1.533 1.533 0 01.947-2.287c1.561-.379 1.561-2.6 0-2.978a1.532 1.532 0 01-.947-2.287c.836-1.372-.734-2.942-2.106-2.106a1.532 1.532 0 01-2.287-.947zM10 13a3 3 0 100-6 3 3 0 000 6z" clip-rule="evenodd" /></svg>
                    ตั้งค่าการเทรด
                </button>
                <button class="settings-btn" id="openSettingsBtn" style="width: 100%; justify-content: flex-start; padding: 10px 14px; font-size: 14px;">
                    <svg viewBox="0 0 20 20" fill="currentColor" style="width:16px; height:16px; margin-right:6px;"><path fill-rule="evenodd" d="M11.49 3.17c-.38-1.56-2.6-1.56-2.98 0a1.532 1.532 0 01-2.286.948c-1.372-.836-2.942.734-2.106 2.106.54.886.061 2.042-.947 2.287-1.561.379-1.561 2.6 0 2.978a1.532 1.532 0 01.947 2.287c-.836 1.372.734 2.942 2.106 2.106a1.532 1.532 0 012.287.947c.379 1.561 2.6 1.561 2.978 0a1.533 1.533 0 012.287-.947c1.372.836 2.942-.734 2.106-2.106a1.533 1.533 0 01.947-2.287c1.561-.379 1.561-2.6 0-2.978a1.532 1.532 0 01-.947-2.287c.836-1.372-.734-2.942-2.106-2.106a1.532 1.532 0 01-2.287-.947zM10 13a3 3 0 100-6 3 3 0 000 6z" clip-rule="evenodd" /></svg>
                    ตั้งค่าการแสดงผล
                </button>
                <button class="settings-btn" id="openSettingsTheresholdBtn" style="width: 100%; justify-content: flex-start; padding: 10px 14px; font-size: 14px;">
                    <svg viewBox="0 0 20 20" fill="currentColor" style="width:16px; height:16px; margin-right:6px;"><path fill-rule="evenodd" d="M11.49 3.17c-.38-1.56-2.6-1.56-2.98 0a1.532 1.532 0 01-2.286.948c-1.372-.836-2.942.734-2.106 2.106.54.886.061 2.042-.947 2.287-1.561.379-1.561 2.6 0 2.978a1.532 1.532 0 01.947 2.287c-.836 1.372.734 2.942 2.106 2.106a1.532 1.532 0 012.287.947c.379 1.561 2.6 1.561 2.978 0a1.533 1.533 0 012.287-.947c1.372.836 2.942-.734 2.106-2.106a1.533 1.533 0 01.947-2.287c1.561-.379 1.561-2.6 0-2.978a1.532 1.532 0 01-.947-2.287c.836-1.372-.734-2.942-2.106-2.106a1.532 1.532 0 01-2.287-.947zM10 13a3 3 0 100-6 3 3 0 000 6z" clip-rule="evenodd" /></svg>
                    ตั้งค่า Thereshold
                </button>
            </div>

            <div style="padding: 20px; border-top: 1px solid rgba(255,255,255,0.08); margin-top: auto;">
                <div style="font-size: 12px; margin-bottom: 8px; color: rgba(255,255,255,0.6);">Theme:</div>
                <div class="theme-selector" style="flex-wrap: wrap;">
                    <button class="theme-btn active" data-theme="lightblue">☀ Light</button>
                    <button class="theme-btn" data-theme="dark">🌙 Dark</button>
                    <button class="theme-btn" data-theme="midnight">✦ Midnight</button>
                </div>
            </div>
            
            <div style="padding: 20px; border-top: 1px solid rgba(255,255,255,0.08); display: flex; flex-direction: column; gap: 8px;" id="tradeControlsPanel">
                <!-- Action buttons will be moved here -->
            </div>
        </aside>

        <!-- ─── MAIN ────────────────────────────────────────────── -->
        <main class="container" style="flex: 1; overflow-y: auto; max-width: none; margin: 0; padding: 20px; position: relative;">"""
content = re.sub(header_pattern, new_layout, content, flags=re.DOTALL)

# 2. Extract Section 1 and Section 2
section_pattern = r'(<!-- ── Section 1: Date Range ─────────────────────────── -->\s*<div class="section-card" id="scheduleSectionCard".*?<!-- ── Tabs ───────────────────────────────────────────── -->)'
match = re.search(section_pattern, content, flags=re.DOTALL)
if match:
    extracted_sections = match.group(1)
    # Remove from main container
    content = content.replace(extracted_sections, '<!-- ── Tabs ───────────────────────────────────────────── -->')
    
    # Extract Go Trade, Stop Trade, Terminate buttons
    buttons_pattern = r'<div style="display:flex;gap:8px;align-self:flex-end;flex-wrap:wrap;">\s*<button class="fetch-btn" id="goTradeBtn">.*?</button>\s*<button class="fetch-btn stop" id="stopTradeBtn">.*?</button>\s*<button class="fetch-btn" id="terminateBtn".*?</button>\s*</div>'
    btn_match = re.search(buttons_pattern, extracted_sections, flags=re.DOTALL)
    if btn_match:
        btns = btn_match.group(0)
        # Modify buttons for the sidebar
        btns_sidebar = btns.replace('display:flex;gap:8px;align-self:flex-end;flex-wrap:wrap;', 'display:flex; flex-direction:column; gap:8px; width:100%;')
        btns_sidebar = btns_sidebar.replace('id="goTradeBtn"', 'id="goTradeBtn" style="width:100%; justify-content:center; padding:12px; font-size:15px; box-shadow:0 4px 12px rgba(26,111,196,0.3);"')
        btns_sidebar = btns_sidebar.replace('id="stopTradeBtn"', 'id="stopTradeBtn" style="width:100%; justify-content:center; padding:12px; font-size:15px;"')
        btns_sidebar = btns_sidebar.replace('id="terminateBtn" style="background:var(--orange);border-color:var(--orange);color:#1a2535;font-weight:700;border:1px solid #c98516;box-shadow:0 2px 4px rgba(0,0,0,0.1);"', 'id="terminateBtn" style="width:100%; justify-content:center; padding:12px; font-size:15px; background:var(--orange); border:1px solid #c98516; color:#1a2535; font-weight:700; box-shadow:0 2px 4px rgba(0,0,0,0.1);"')
        
        # Remove buttons from the modal content
        extracted_sections = extracted_sections.replace(btns, '')
        
        # Inject buttons into the Left Panel
        content = content.replace('<!-- Action buttons will be moved here -->', btns_sidebar)

    # Wrap extracted sections into a new Modal
    trade_settings_modal = f"""
    <!-- ─── TRADE SETTINGS MODAL ──────────────────────────────────── -->
    <div class="modal" id="tradeSettingsModal">
        <div class="modal-content" style="max-width: 900px;">
            <div class="modal-header">
                <div class="modal-title">
                    <div class="modal-title-icon">⚙</div>ตั้งค่าการเทรด (Trade Settings)
                </div>
                <button class="modal-close" id="closeTradeSettingsBtn">×</button>
            </div>
            <div class="modal-body">
                {extracted_sections.replace('<!-- ── Tabs ───────────────────────────────────────────── -->', '')}
            </div>
        </div>
    </div>
    """
    
    # Inject the new modal just before the existing SETTINGS MODAL
    content = content.replace('<!-- ─── SETTINGS MODAL ──────────────────────────────────── -->', trade_settings_modal + '\n    <!-- ─── SETTINGS MODAL ──────────────────────────────────── -->')

# 3. Close the layout div before closing body
content = content.replace('</body>', '    </div>\n</body>')

with open("D:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/public/index.html", "w", encoding="utf-8") as f:
    f.write(content)
print("done")
