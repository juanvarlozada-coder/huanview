# Huanview

Visor de imágenes minimalista, rápido y fluido, escrito en **Rust** (eframe/egui), inspirado en la
arquitectura de Gwenview (analizada con REA + Hopper sobre `/usr/bin/gwenview` y
`/usr/lib/libgwenviewlib.so.5`).

## Rendimiento
Decodificación medida (release, optimización LTO): **< 120 ms en el peor caso** para imágenes
de 2K en disco local. Objetivo «cargar en menos de 1 s» cumplido con margen.

## Arquitectura (patrones de Gwenview observados por REA)
| Gwenview | Huanview |
|---|---|
| `LoadingJob` / `ThreadedDocumentJob` | Pool de workers + canal de trabajos, la UI nunca decodifica |
| `DownSamplingJob` | Miniaturas decodificadas y reducidas en el pool (filmstrip) |
| `DocumentViewContainer` (vistas apiladas, transiciones) | Animación de zoom/paneo con *lerp* acotado + fade-in de navegación |
| Preload de vecinos | Cola de prioridad: actual → siguiente → anterior |
| `ThumbnailGenerator` / `ThumbnailWriter` | Miniaturas 128 px en memoria (LRU 600) |
| `AnnotateDialog` + herramientas rastreables | Herramientas Lápiz / Marcador sobre el lienzo |
| HUD (`HudButtonBox`, `HudMessageBubble`) | Chips translúcidos con atajos, colores y borrar |

## Uso
```sh
huanview ruta/a/imagen.jpg    # abre imagen, navega su carpeta
huanview ruta/a/carpeta       # abre carpeta completa
```

## Atajos
| Tecla | Acción |
|---|---|
| ← / → | anterior / siguiente imagen |
| Inicio / Fin | primera / última |
| + / − | zoom animado |
| 1 / 2 | ajustar a ventana / 1:1 píxeles reales |
| F | pantalla completa |
| A / M / V | lápiz / marcador / modo ver (paneo) |
| Supr | borrar anotaciones de la imagen |
| Ctrl+S | guardar anotaciones en un `.huanview.json` junto a la imagen (no destructivo) |

Además: rueda = zoom anclado al cursor · arrastre = paneo (latencia cero) · doble clic
= alternar ajuste/2.5x · clic en bordes = navegar (imagen ajustada).

## Instalación (ya aplicada)
- Binario: `~/.local/bin/huanview`
- Entrada de menú: `~/.local/share/applications/huanview.desktop` (Graphics/Viewer)
- Ícono: `~/.local/share/icons/hicolor/256x256/apps/huanview.png`

Validado con `desktop-file-validate`. Para dejarlo como visor predeterminado:
```sh
xdg-mime default huanview.desktop image/jpeg image/png image/webp
```

## Publicación
Paquetes listos en `dist/` (y autogenerables con GitHub Actions en `.github/workflows/build.yml`):
- `Huanview-0.1.0-installer-win64.exe` — instalador NSIS para Windows x64 (accesos directos, desinstalador, HKCU)
- `Huanview-0.1.0-portable-win64.zip` — versión portable de Windows
- `Huanview-0.1.0-x86_64.AppImage` — Linux (X11 y Wayland), empacado con linuxdeploy + appimagetool

Para reproducir el AppImage localmente:
```sh
cargo build --release
linuxdeploy --appdir AppDir --executable AppDir/usr/bin/huanview \
  --desktop-file AppDir/huanview.desktop --icon-file AppDir/huanview.png
appimagetool AppDir Huanview-0.1.0-x86_64.AppImage
```

## Compilar
```sh
cargo build --release
```
