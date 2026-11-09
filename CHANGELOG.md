# Changelog de Huanview

## v0.1.7 — Interfaz reorganizada y temas
- **Nombre del archivo** centrado entre la imagen y la tira de miniaturas (con contador i/<total>), ya no en la barra superior.
- **Menú hamburguesa ☰ a la izquierda**: opciones de archivo (Abrir, Guardar, Guardar como), **Copiar imagen al portapapeles**, buscar actualizaciones y abrir ajustes — ahora compacto y ordenado.
- **Herramientas de dibujo a la derecha**, separadas del menú: Vista, Lápiz, Marcador, colores y Borrar.
- **Ajustes con tamaño fijo** (560×340) sin importar la categoría activa.
- **Tema claro/oscuro** conmutable en Ajustes → General (se recuerda entre sesiones).

## v0.1.6 — Atajos de teclado personalizables
- **Ajustes → ⌨ Atajos**: haz clic en una combinación y pulsa la nueva tecla para reasignarla (Esc cancela). Se guarda en `~/.config/huanview/settings.json`.
- Acciones configurables: **Copiar imagen** (Ctrl+C), **Deshacer** (Ctrl+Z), **Rehacer** (Ctrl+Y), herramientas **Vista/Lápiz/Marcador** (V/A/M) y **Abrir ajustes** (Ctrl+,).
- **Copiar al portapapeles**: el atajo copia la imagen actual al portapapeles del sistema (Windows/Wayland/X11).
- **Deshacer y rehacer** anotaciones con historial de hasta 60 pasos.
- **Guardar como** ahora también con Ctrl+Shift+S.
- **Mano temporal**: mantén **Espacio** para desplazarte cómodamente aunque tengas lápiz o marcador activos (fijo).

## v0.1.5 — Correcciones y refino
- **Bug visual corregido**: el marco azul de selección de las miniaturas ya no se recorta por los bordes de la tira de imágenes (margen interior + trazado del contorno dentro del recuadro).
- **Idiomas**: la interfaz ahora sí se actualiza al instante al cambiar de idioma (el fallo venía del antiguo menú desplegable, que se tragaba los clics del selector).
- **Buscar actualizaciones manualmente**: nuevo botón ⌗/⟳ en la barra superior (junto al engranaje) que lanza el chequeo al momento, además del que ya existía dentro de Ajustes → Actualizaciones.

## v0.1.4 — Iconos y preferencias organizadas
- **Iconos vectoriales** (dibujados con el propio motor, escalan perfecto y sin fuentes) en la barra superior y el chip de zoom: lápiz ✏️, marcador 🖊️, borrador, carpeta (abrir), disquete con + (guardar como), lupa +/− para el zoom, marco de ajuste y engranaje de preferencias; al pasar el cursor aparece un tooltip explicativo en tu idioma.
- **Ajustes en ventana**, ya no desplegable: se abre al pulsar el engranaje, con las preferencias **organizadas por categorías**:
  - 🗂 General (idioma)
  - 🔄 Actualizaciones (chequeo automático + botón "Buscar ahora")
  - ℹ️ Acerca de (versión + novedades/changelog)

## v0.1.3 — Multi-idioma
- **Idiomas en la app**: Español, English y Português (⚙ → Idioma; se recuerda en `~/.config/huanview/settings.json`, con detección automática del idioma del sistema la primera vez).
- **Todos los textos de la interfaz** traducidos: herramientas, archivos, avisos (descargas, anotaciones, errores) y ajustes.
- **Instalador de Windows multi-idioma**: diálogo de elección (Español / English / Português), accesos directos y mensajes de instalación/desinstalación traducidos.

## v0.1.2 — Interfaz renovada
- **Flechas de navegación** grandes y translúcidas sobre la imagen: aparecen al pasar el cursor y se atenúan en el primer/último fotograma.
- **Barra superior fija** (en lugar del panel flotante): herramientas de dibujo (Vista, Lápiz, Marcador, colores, Borrar) y opciones de archivo (Abrir, Guardar, Guardar como).
- **Abrir** (Ctrl+O): diálogo nativo para cargar otra imagen/carpeta.
- **Guardar**: exporta las anotaciones como archivo lateral `.huanview.json` (igual que Ctrl+S).
- **Guardar como**: exporta la imagen con las anotaciones incrustadas en un PNG nuevo.
- **Controles de zoom** en un chip compacto en la esquina inferior derecha: −, porcentaje, Ajustar, +.
- **Configuración (⚙)** en la barra: versión del programa, novedades (este changelog) e interruptor de "Buscar actualizaciones al iniciar".

## v0.1.1 — Auto-actualizador
- Chequeo automático de nuevas versiones al iniciar (hilo aparte, sin bloquear la UI).
- Botón Instalar + barra de progreso; Windows reinstala silencioso (`/S`), Linux hace swap atómico del AppImage.
- Configuración opcional en `~/.config/huanview/updater.json` (`auto_check`).

## v0.1.0 — Primera versión
- Visor minimalista en Rust (eframe/egui) inspirado en Gwenview.
- Carga en paralelo con cola de prioridad (actual → siguiente → anterior) y cancelación por generación.
- Zoom/pan suaves, doble clic fit↔2.5×, rueda para zoom anclado al cursor.
- Filmstrip de miniaturas abajo; anotaciones a mano alzada (lápiz/marcador, Ctrl+S exportal lateral).
- Empaquetado: instalador NSIS y portable para Windows, AppImage para Linux.
