import os

svg_templates = {
    'readyplayer.me.png': '''<svg xmlns="http://www.w3.org/2000/svg" width="400" height="400" viewBox="0 0 400 400">
      <defs>
        <radialGradient id="bg" cx="50%" cy="40%" r="60%">
          <stop offset="0%" stop-color="#1e293b"/>
          <stop offset="100%" stop-color="#090d16"/>
        </radialGradient>
        <linearGradient id="glow" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#38bdf8"/>
          <stop offset="100%" stop-color="#818cf8"/>
        </linearGradient>
      </defs>
      <rect width="400" height="400" fill="url(#bg)"/>
      <circle cx="200" cy="150" r="60" fill="url(#glow)" opacity="0.9"/>
      <ellipse cx="200" cy="300" rx="90" ry="70" fill="url(#glow)" opacity="0.8"/>
      <text x="200" y="380" font-family="sans-serif" font-size="18" font-weight="bold" fill="#38bdf8" text-anchor="middle">ReadyPlayerMe Humanoid</text>
    </svg>''',
    'Soldier.png': '''<svg xmlns="http://www.w3.org/2000/svg" width="400" height="400" viewBox="0 0 400 400">
      <defs>
        <radialGradient id="bg" cx="50%" cy="40%" r="60%">
          <stop offset="0%" stop-color="#134e4a"/>
          <stop offset="100%" stop-color="#090d16"/>
        </radialGradient>
      </defs>
      <rect width="400" height="400" fill="url(#bg)"/>
      <polygon points="200,90 260,150 240,250 160,250 140,150" fill="#14b8a6"/>
      <rect x="175" y="110" width="50" height="15" rx="4" fill="#0f172a"/>
      <rect x="155" y="250" width="90" height="60" rx="8" fill="#0d9488"/>
      <text x="200" y="380" font-family="sans-serif" font-size="18" font-weight="bold" fill="#2dd4bf" text-anchor="middle">Combat Soldier 3D Rig</text>
    </svg>''',
    'celestia.png': '''<svg xmlns="http://www.w3.org/2000/svg" width="400" height="400" viewBox="0 0 400 400">
      <defs>
        <radialGradient id="bg" cx="50%" cy="40%" r="60%">
          <stop offset="0%" stop-color="#4c1d95"/>
          <stop offset="100%" stop-color="#090d16"/>
        </radialGradient>
        <linearGradient id="wings" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#c084fc"/>
          <stop offset="100%" stop-color="#f472b6"/>
        </linearGradient>
      </defs>
      <rect width="400" height="400" fill="url(#bg)"/>
      <!-- Wings -->
      <path d="M200,180 Q100,80 40,130 Q90,210 200,220" fill="url(#wings)" opacity="0.75"/>
      <path d="M200,180 Q300,80 360,130 Q310,210 200,220" fill="url(#wings)" opacity="0.75"/>
      <!-- Body -->
      <circle cx="200" cy="140" r="45" fill="#fbcfe8"/>
      <ellipse cx="200" cy="270" rx="60" ry="75" fill="url(#wings)"/>
      <!-- Halo -->
      <ellipse cx="200" cy="85" rx="50" ry="12" fill="none" stroke="#fbbf24" stroke-width="6"/>
      <text x="200" y="380" font-family="sans-serif" font-size="18" font-weight="bold" fill="#f472b6" text-anchor="middle">Celestia (Star Goddess)</text>
    </svg>''',
    'paladin.png': '''<svg xmlns="http://www.w3.org/2000/svg" width="400" height="400" viewBox="0 0 400 400">
      <defs>
        <radialGradient id="bg" cx="50%" cy="40%" r="60%">
          <stop offset="0%" stop-color="#78350f"/>
          <stop offset="100%" stop-color="#090d16"/>
        </radialGradient>
      </defs>
      <rect width="400" height="400" fill="url(#bg)"/>
      <!-- Helmet -->
      <path d="M160,110 Q200,70 240,110 L245,180 Q200,205 155,180 Z" fill="#e2e8f0"/>
      <rect x="175" y="130" width="50" height="10" rx="3" fill="#0f172a"/>
      <!-- Shield / Plate -->
      <path d="M140,190 L260,190 L250,290 L200,325 L150,290 Z" fill="#94a3b8"/>
      <!-- Cross -->
      <rect x="193" y="215" width="14" height="70" fill="#f59e0b"/>
      <rect x="170" y="235" width="60" height="14" fill="#f59e0b"/>
      <text x="200" y="380" font-family="sans-serif" font-size="18" font-weight="bold" fill="#fbbf24" text-anchor="middle">Paladin (Holy Sentinel)</text>
    </svg>''',
    'kira.png': '''<svg xmlns="http://www.w3.org/2000/svg" width="400" height="400" viewBox="0 0 400 400">
      <defs>
        <radialGradient id="bg" cx="50%" cy="40%" r="60%">
          <stop offset="0%" stop-color="#831843"/>
          <stop offset="100%" stop-color="#090d16"/>
        </radialGradient>
      </defs>
      <rect width="400" height="400" fill="url(#bg)"/>
      <!-- Anime Cyberpunk Silhouette -->
      <circle cx="200" cy="140" r="45" fill="#fecdd3"/>
      <!-- Cyber Visor -->
      <polygon points="165,130 235,130 225,145 175,145" fill="#ec4899"/>
      <!-- Hair -->
      <path d="M150,140 Q140,80 200,80 Q260,80 250,140 Q270,180 260,210 Q240,160 235,150 Q165,150 160,160 Q130,180 150,140" fill="#f43f5e"/>
      <path d="M150,210 L250,210 L240,320 L160,320 Z" fill="#0f172a"/>
      <text x="200" y="380" font-family="sans-serif" font-size="18" font-weight="bold" fill="#fb7185" text-anchor="middle">Kira Anime Cyberpunk</text>
    </svg>''',
    'RobotExpressive.png': '''<svg xmlns="http://www.w3.org/2000/svg" width="400" height="400" viewBox="0 0 400 400">
      <defs>
        <radialGradient id="bg" cx="50%" cy="40%" r="60%">
          <stop offset="0%" stop-color="#1e3a8a"/>
          <stop offset="100%" stop-color="#090d16"/>
        </radialGradient>
      </defs>
      <rect width="400" height="400" fill="url(#bg)"/>
      <!-- Antenna -->
      <line x1="200" y1="90" x2="200" y2="60" stroke="#38bdf8" stroke-width="6"/>
      <circle cx="200" cy="55" r="8" fill="#38bdf8"/>
      <!-- Robot Head -->
      <rect x="150" y="90" width="100" height="80" rx="16" fill="#64748b"/>
      <!-- Visor / Eyes -->
      <rect x="165" y="110" width="70" height="30" rx="8" fill="#0284c7"/>
      <circle cx="185" cy="125" r="7" fill="#38bdf8"/>
      <circle cx="215" cy="125" r="7" fill="#38bdf8"/>
      <!-- Body -->
      <rect x="140" y="190" width="120" height="110" rx="12" fill="#475569"/>
      <circle cx="200" cy="235" r="18" fill="#38bdf8"/>
      <text x="200" y="380" font-family="sans-serif" font-size="18" font-weight="bold" fill="#38bdf8" text-anchor="middle">Robot Expressive</text>
    </svg>''',
    'Avatar_Orion.png': '''<svg xmlns="http://www.w3.org/2000/svg" width="400" height="400" viewBox="0 0 400 400">
      <defs>
        <radialGradient id="bg" cx="50%" cy="40%" r="60%">
          <stop offset="0%" stop-color="#312e81"/>
          <stop offset="100%" stop-color="#090d16"/>
        </radialGradient>
      </defs>
      <rect width="400" height="400" fill="url(#bg)"/>
      <!-- Cosmic Stars -->
      <circle cx="90" cy="90" r="3" fill="#fff"/>
      <circle cx="310" cy="100" r="2.5" fill="#fff"/>
      <circle cx="120" cy="310" r="2" fill="#fff"/>
      <!-- Humanoid Silhouette -->
      <circle cx="200" cy="130" r="45" fill="#e0e7ff"/>
      <path d="M150,195 L250,195 L240,320 L160,320 Z" fill="#4338ca"/>
      <!-- Celestial Aura -->
      <circle cx="200" cy="130" r="60" fill="none" stroke="#818cf8" stroke-width="4" stroke-dasharray="8 6"/>
      <text x="200" y="380" font-family="sans-serif" font-size="18" font-weight="bold" fill="#818cf8" text-anchor="middle">Orion (Celestial Guardian)</text>
    </svg>'''
}

thumb_dir = 'showcase/public/thumbnails'
for name, svg in svg_templates.items():
    svg_path = os.path.join(thumb_dir, name.replace('.png', '.svg'))
    with open(svg_path, 'w', encoding='utf-8') as f:
        f.write(svg)
    print(f"Created {svg_path}")

print("All SVG thumbnails created.")
