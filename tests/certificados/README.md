# Certificados de prueba

Estos archivos son **solo para los tests**:

- `ca.pem`: CA autofirmada de prueba (`CN=Vaca Test CA`).
- `servidor.pem`: certificado de servidor firmado por esa CA, con SAN
  `localhost` y `127.0.0.1`.
- `servidor-clave.pem`: clave privada del certificado de servidor.

No los uses en producción. Para un servidor real genera un certificado con una
CA pública o interna y guarda la clave con permisos `0600`.
