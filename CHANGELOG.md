# Changelog de Huanview

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
