# **Set-Location "$env:USERPROFILE\\Desktop\\Low-GT-VI"**

# 

# **$readme = @"**

# **# Low GT VI**

# 

# **Fork optimise et stabilise pour configurations modestes (\*\*AMD Radeon RX 6600 XT 8 Go\*\*, \*\*Ryzen 7 5700G\*\*, \*\*16 Go RAM DDR4\*\*).**

# 

# **## Etat d'avancement (Status)**

# 

# **| Etape | Statut | Details / Notes |**

# **| :--- | :---: | :--- |**

# **| \*\*Boot / Demarrage\*\* | ✅ Fonctionnel | Lancement propre, chargement des archives RPF et initialisation D3D12 |**

# **| \*\*Menus \& Calibrage\*\* | ✅ Fonctionnel | Interface fluide (30 FPS), reglage luminosite et navigation stables |**

# **| \*\*Ecran de chargement\*\* | ✅ Fonctionnel | Passe completement sans blocage ni crash GPU AMD (`0x887A0007` corrige) |**

# **| \*\*Cinematique d'intro\*\* | ✅ Stable | Tourne sans crash (`VICTORY\\\_RELAY=0`), pointes jusqu'a \*\*\~8-10 FPS\*\* |**

# **| \*\*Gameplay / En jeu\*\* | 🚧 Jouable (WIP) | Entre en jeu avec des pointes jusqu'a \*\*10 FPS\*\* sur 16 Go RAM (optimisation memoire en cours) |**

# 

# **# Poor VI**

# 

# **\*\*Xbox Series compatibility on Windows, tuned for AMD RDNA2 (RX 6600 XT 8 GB) \& 16 GB RAM.\*\***

# 

# **Poor VI is a community compatibility project built upon VIctory and \[TaynanGT/Vice-Tay](https://github.com/TaynanGT/Vice-Tay) to run Xbox Series titles on mid-range PC hardware (`Ryzen 7 5700G` + `RX 6600 XT 8 GB` + `16 GB RAM`).** 

# 

# **---**

# 

# **## Statut \& Mesures Clés**

# 

# **\* \*\*Menus \& Calibrage :\*\* 30.0 FPS stables (`33.3 ms/frame`)**

# **\* \*\*Écran de chargement / Cinématiques :\*\* Fonctionnel sans plantage UAV RDNA2 (`VICTORY\_RELAY='0'`) avec des pics de 8 à 10 FPS.**

# **\* \*\*Gameplay :\*\* Jouable (WIP) avec des pics à \~10 FPS sur 16 GB DDR4 (pagefile actif).**

# 

# **### Configuration de Test**

# **\* \*\*CPU :\*\* AMD Ryzen 7 5700G (8 cœurs / 16 threads)**

# **\* \*\*GPU :\*\* AMD Radeon RX 6600 XT (8 GB GDDR6, RDNA2)**

# **\* \*\*RAM :\*\* 16 GiB DDR4 + fichier d'échange SSD**

# **\* \*\*OS :\*\* Windows 10/11 x64**

# 

# **---**

# 

# **## Démarrage Rapide**

# 

# **### 1. Build**

# **```powershell**

# **cargo build --locked --release --features diagnostics -p victory**

# **```**

# 

# **### 2. Lancement (Profil sécurisé RDNA2 / 16 GB RAM)**

# **Utilise le script PowerShell avec le profil optimisé :**

# **```powershell**

# **.\\scripts\\play.ps1 -Exe 'C:\\(\\path\\to\\game.\\)exe' -Console series-s -Build release -More @{**

# &#x20;   **VICTORY\_RELAY = '0'**

# &#x20;   **VICTORY\_TEXTURE\_MAX = '512'**

# &#x20;   **VICTORY\_CARD\_BUFFERS\_MB = '2048'**

# **}**

# **```**

# **\*(Le script complet incluant les variables d'environnement détaillées se trouve dans le bloc PowerShell principal de la réponse source).\***

# 

# **---**

# 

# **## Crédits \& Licence**

# **\* \*\*Bases \& Architecture :\*\* Inspiré et construit à partir de \[TaynanGT/Vice-Tay](https://github.com/TaynanGT/Vice-Tay), \*\*Marvisak/VIctory\*\* et des contributeurs AMD/RDNA2 (`avatarmanz`, `codec-d`, `Belobug`, etc.).**

# **\* \*\*Licence :\*\* GPL-3.0-or-later.**



