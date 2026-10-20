# Empaquetado y despliegue de Vaca

Cargo se usa para **construir** el binario. El usuario final no necesita Rust,
Cargo ni el código fuente: recibe un ejecutable nativo.

## Opción 1: tarball con instalador

```bash
packaging/empaquetar.sh
```

Genera:

```text
dist/
  vaca-0.1.0-x86_64-unknown-linux-gnu/
    bin/vaca
    install.sh
    README.md
    LICENSE
    share/vaca/examples/
  vaca-0.1.0-x86_64-unknown-linux-gnu.tar.gz
  vaca-0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256
```

Instalación en el usuario actual:

```bash
tar -xzf dist/vaca-0.1.0-x86_64-unknown-linux-gnu.tar.gz
cd vaca-0.1.0-x86_64-unknown-linux-gnu
./install.sh
# instala en ~/.local/bin/vaca
```

Instalación global:

```bash
PREFIX=/usr/local sudo ./install.sh
```

También puedes copiar solo el binario:

```bash
sudo install -m 755 bin/vaca /usr/local/bin/vaca
```

## Opción 2: binario estático (musl)

Para Linux sin depender de la versión de glibc:

```bash
rustup target add x86_64-unknown-linux-musl
packaging/empaquetar.sh --target x86_64-unknown-linux-musl
```

El binario resultante puede ejecutarse en distribuciones Linux con una libc
compatible y es adecuado para imágenes `scratch` o `distroless`.

## Opción 3: Docker

Desde la raíz del proyecto:

```bash
docker build -f packaging/Dockerfile -t vaca:0.1.0 .
docker run --rm -p 8080:8080 vaca:0.1.0
```

La imagen ejecuta `ejemplos/servidor.vaca` por defecto. Para empaquetar una app
propia, cambia `CMD` o monta el código:

```bash
docker run --rm -p 8080:8080 \
  -v "$PWD/mi_app.vaca:/app/app.vaca:ro" \
  -w /app \
  vaca:0.1.0 /app/app.vaca
```

El servidor HTTP de Vaca escucha en `0.0.0.0` cuando se usa
`http.servidor(puerto, manejador)`.

Si quieres una imagen más pequeña, compila un binario estático con musl y usa:

```dockerfile
FROM scratch
COPY vaca /vaca
COPY app.vaca /app.vaca
ENTRYPOINT ["/vaca", "/app.vaca"]
```

## Opción 4: systemd

Estructura recomendada:

```text
/opt/vaca/
  app.vaca
  data/
/usr/local/bin/vaca
```

Pasos:

```bash
sudo useradd --system --home /opt/vaca --shell /usr/sbin/nologin vaca
sudo mkdir -p /opt/vaca/data
sudo cp mi_app.vaca /opt/vaca/app.vaca
sudo cp target/release/vaca /usr/local/bin/vaca
sudo cp packaging/vaca.service /etc/systemd/system/vaca.service
sudo chown -R vaca:vaca /opt/vaca
sudo systemctl daemon-reload
sudo systemctl enable --now vaca
sudo systemctl status vaca
```

Logs:

```bash
journalctl -u vaca -f
```

El servicio aplica `ProtectSystem=strict` y solo permite escritura en
`/opt/vaca/data`. Ajusta `ReadWritePaths` si tu aplicación escribe en otras
rutas.

## Empaquetar una aplicación Vaca

Vaca todavía ejecuta archivos `.vaca`, así que una aplicación necesita:

```text
app/
  bin/vaca
  app.vaca
  datos/
  assets/
```

Opciones:

- **Tarball**: incluye el binario y los archivos de la app.
- **Docker**: es la forma más cómoda de obtener un único artefacto desplegable.
- **Binario con fuente incrustada**: un lanzador Rust puede usar
  `include_str!("app.vaca")` y la API `vaca::Interprete`. Es útil si quieres un
  ejecutable único sin Docker, pero hay que compilar un pequeño binario por
  aplicación.

Un comando futuro `vaca empaquetar app.vaca` podría generar ese lanzador
automáticamente.

## Cross-compilación

Rust compila para otros targets:

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

Para Windows:

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

Para macOS necesitas compilar en macOS o usar una toolchain cruzada.

## Verificación

```bash
cd dist
sha256sum -c vaca-*.tar.gz.sha256
cd ..
tar -tzf dist/vaca-*.tar.gz
```

Comprueba también el binario empaquetado:

```bash
./dist/vaca-0.1.0-x86_64-unknown-linux-gnu/bin/vaca --version
./dist/vaca-0.1.0-x86_64-unknown-linux-gnu/bin/vaca \
  ./dist/vaca-0.1.0-x86_64-unknown-linux-gnu/share/vaca/examples/hola.vaca
```
