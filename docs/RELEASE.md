# Publicar instaladores (desativado)

Hoje o projeto **não publica binários**: quem quiser usar compila a própria cópia (veja o README). Por isso o workflow de release não está ativo e o atualizador automático do app fica desligado.

Este documento fica como referência para o dia em que isso mudar. Para reativar, copie `docs/release.yml.example` para `.github/workflows/release.yml` e siga os passos abaixo.

## Uma vez só: chave do atualizador

O app se atualiza pelo GitHub Releases. Cada pacote de atualização é assinado com uma chave sua, e o app só instala o que bater com a chave pública embutida nele. Enquanto a chave não for configurada, o atualizador fica desligado e o resto do app funciona normalmente.

1. Gere o par de chaves (defina uma senha quando pedir):

   ```bash
   npm run tauri signer generate -- -w ~/.tauri/funkdown.key
   ```

2. Copie o conteúdo de `~/.tauri/funkdown.key.pub` para o campo `plugins.updater.pubkey` de `src-tauri/tauri.conf.json`, no lugar do texto `SUBSTITUA_...`.

3. No GitHub, em **Settings → Secrets and variables → Actions**, crie:
   - `TAURI_SIGNING_PRIVATE_KEY`: o conteúdo de `~/.tauri/funkdown.key`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: a senha da chave

Guarde a chave privada fora do repositório e com backup. Se ela se perder, as versões já instaladas não aceitam mais atualizações e cada pessoa precisa reinstalar à mão.

O endereço de atualização em `tauri.conf.json` aponta para `GianFadiga/FunkDown`. Renomeie o repositório no GitHub (**Settings → General → Repository name**) ou ajuste o endereço.

## A cada versão

1. Atualize a versão nos três lugares: `Cargo.toml` (`[workspace.package]`), `package.json` e `src-tauri/tauri.conf.json`.
2. Faça o commit e crie a tag:

   ```bash
   git commit -am "v2.0.1"
   git tag v2.0.1
   git push && git push --tags
   ```

3. O workflow **Release** gera os instaladores de Windows e Linux, o `latest.json` do atualizador e um rascunho de release.
4. Revise o rascunho no GitHub e publique. A partir daí, os apps instalados oferecem a atualização na próxima abertura.

## Preservar a versão Python

Antes do primeiro commit da v2, marque o estado antigo do repositório:

```bash
git tag v1-python
git branch legacy-python
git push origin v1-python legacy-python
```
