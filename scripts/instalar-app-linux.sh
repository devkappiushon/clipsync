#!/usr/bin/env bash
# Instala o AppImage do ClipSync para o usuário atual, sem sudo (contrato §12.8.5).
#   scripts/instalar-app-linux.sh caminho/clipsync-app-vX.Y.Z-linux-x86_64.AppImage
# Copia para ~/.local/bin/clipsync-app.AppImage, extrai o ícone do próprio AppImage
# e grava ~/.local/share/applications/clipsync.desktop. Rodar de novo atualiza.
set -euo pipefail

origem=${1:?uso: $0 caminho/do/clipsync-app.AppImage}
[ -f "$origem" ] || { echo "não achei $origem" >&2; exit 1; }

bin_dir="$HOME/.local/bin"
apps_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
icon_dir="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps"
destino="$bin_dir/clipsync-app.AppImage"

mkdir -p "$bin_dir" "$apps_dir" "$icon_dir"
install -m 0755 "$origem" "$destino.novo"
mv -f "$destino.novo" "$destino"

# O AppDir do Tauri leva o ícone maior como ClipSync.png na raiz.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
( cd "$tmp" && "$destino" --appimage-extract ClipSync.png > /dev/null )
icone="$tmp/squashfs-root/ClipSync.png"
[ -s "$icone" ] || { echo "o AppImage não trouxe ClipSync.png" >&2; exit 1; }
install -m 0644 "$icone" "$icon_dir/clipsync.png"

cat > "$apps_dir/clipsync.desktop.novo" <<DESKTOP
[Desktop Entry]
Type=Application
Name=ClipSync
Comment=Teclado, mouse e área de transferência entre os seus computadores
Exec="$destino"
Icon=clipsync
Terminal=false
Categories=Utility;
StartupNotify=true
DESKTOP
mv -f "$apps_dir/clipsync.desktop.novo" "$apps_dir/clipsync.desktop"

command -v update-desktop-database > /dev/null && update-desktop-database "$apps_dir" || true
command -v gtk-update-icon-cache > /dev/null && gtk-update-icon-cache -q -t "${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor" || true

echo "instalado: $destino"
echo "atalho: $apps_dir/clipsync.desktop"
