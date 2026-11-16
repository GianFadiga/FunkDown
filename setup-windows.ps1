<#
.SYNOPSIS
  Prepara o Windows e roda o FunkDown em modo de desenvolvimento.

.DESCRIPTION
  1. Instala o que faltar: Rust, Node.js LTS, Build Tools do Visual Studio (C++) e WebView2.
  2. Copia o projeto para fora do OneDrive (padrão: C:\dev\FunkDown).
  3. Renomeia _github para .github.
  4. Roda `npm install` e abre o app (`npm run tauri dev`).

  Pode ser executado de novo sem problema: o que já está instalado é pulado.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File .\setup-windows.ps1

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File .\setup-windows.ps1 -Build
  Gera o instalador (.exe) em vez de abrir o app.
#>
param(
    [string]$Destino = "C:\dev\FunkDown",
    [switch]$Build,
    [switch]$SemCopia
)

$ErrorActionPreference = "Stop"

function Passo($texto) { Write-Host "`n==> $texto" -ForegroundColor Cyan }
function Ok($texto) { Write-Host "    $texto" -ForegroundColor Green }
function Falha($texto) { Write-Host "`nERRO: $texto" -ForegroundColor Red; exit 1 }
function Existe($comando) { [bool](Get-Command $comando -ErrorAction SilentlyContinue) }

# Programas recém-instalados só entram no PATH de terminais novos; isto recarrega o PATH aqui mesmo.
function Atualizar-Path {
    $env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" +
                [Environment]::GetEnvironmentVariable("Path", "User") + ";" +
                "$env:USERPROFILE\.cargo\bin"
}

function Instalar($id, $nome, $extra = @()) {
    Write-Host "    Instalando $nome (pode pedir permissão de administrador)..."
    $argumentos = @("install", "--id", $id, "-e", "--accept-source-agreements", "--accept-package-agreements") + $extra
    & winget @argumentos
    Atualizar-Path
}

function Tem-BuildTools {
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $vswhere)) { return $false }
    $caminho = & $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    return [bool]$caminho
}

function Tem-WebView2 {
    $guid = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
    foreach ($raiz in "HKLM:\SOFTWARE\WOW6432Node", "HKLM:\SOFTWARE", "HKCU:\SOFTWARE") {
        if (Test-Path "$raiz\Microsoft\EdgeUpdate\Clients\$guid") { return $true }
    }
    return $false
}

# ---------------------------------------------------------------- pré-requisitos
Passo "Conferindo pré-requisitos"
if (-not (Existe winget)) {
    Falha "O winget não foi encontrado. Instale o 'Instalador de Aplicativo' pela Microsoft Store e rode o script de novo."
}
Atualizar-Path

if (Tem-BuildTools) { Ok "Build Tools do Visual Studio (C++): ok" }
else {
    # São alguns GB; a janela do instalador mostra o progresso.
    Instalar "Microsoft.VisualStudio.2022.BuildTools" "Build Tools do Visual Studio (C++)" `
        @("--override", "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended")
    if (-not (Tem-BuildTools)) {
        Falha "As Build Tools não foram instaladas. Abra o 'Visual Studio Installer', marque 'Desenvolvimento para desktop com C++' e rode o script de novo."
    }
}

if (Existe cargo) { Ok "Rust: $(rustc --version)" }
else {
    Instalar "Rustlang.Rustup" "Rust (rustup)"
    if (-not (Existe rustup)) { Falha "O Rust não foi instalado. Instale por https://rustup.rs e rode o script de novo." }
    & rustup default stable-x86_64-pc-windows-msvc
    if (-not (Existe cargo)) { Falha "O Rust foi instalado, mas o 'cargo' não apareceu. Feche este terminal, abra outro e rode o script de novo." }
    Ok "Rust: $(rustc --version)"
}

if (Existe node) { Ok "Node.js: $(node --version)" }
else {
    Instalar "OpenJS.NodeJS.LTS" "Node.js LTS"
    if (-not (Existe node)) { Falha "O Node.js foi instalado, mas não apareceu no PATH. Feche este terminal, abra outro e rode o script de novo." }
    Ok "Node.js: $(node --version)"
}
$versaoNode = [int]((node --version) -replace "^v(\d+)\..*", '$1')
if ($versaoNode -lt 20) { Falha "O Node.js instalado é o $versaoNode; o projeto precisa do 20 ou mais novo (recomendado: 22)." }

if (Tem-WebView2) { Ok "WebView2: ok" }
else { Instalar "Microsoft.EdgeWebView2Runtime" "WebView2" }

# ---------------------------------------------------------------- cópia do projeto
$origem = $PSScriptRoot
if (-not (Test-Path (Join-Path $origem "src-tauri\tauri.conf.json"))) {
    Falha "Rode este script de dentro da pasta do projeto (onde ficam package.json e src-tauri)."
}

$mesmaPasta = (Test-Path $Destino) -and ((Resolve-Path $Destino).Path.TrimEnd("\") -eq (Resolve-Path $origem).Path.TrimEnd("\"))
if ($SemCopia -or $mesmaPasta) {
    $Destino = $origem
    if ($Destino -like "*OneDrive*") {
        Write-Host "    Aviso: compilar dentro do OneDrive é lento e pode travar arquivos." -ForegroundColor Yellow
    }
}
else {
    Passo "Copiando o projeto para $Destino"
    # node_modules, target e dist são recriados pelo build; não vale copiar.
    # /XO: não sobrescreve arquivos que você já editou na pasta de destino.
    & robocopy $origem $Destino /E /XO /XD node_modules target dist .git /NFL /NDL /NJH /NJS /NP | Out-Null
    if ($LASTEXITCODE -ge 8) { Falha "A cópia falhou (robocopy devolveu $LASTEXITCODE)." }
    Ok "Projeto copiado."
}

Set-Location $Destino
if ((Test-Path "_github") -and -not (Test-Path ".github")) {
    Rename-Item "_github" ".github"
    Ok "_github renomeada para .github"
}

# ---------------------------------------------------------------- dependências e execução
Passo "Instalando as dependências da interface (npm install)"
& npm install
if ($LASTEXITCODE -ne 0) { Falha "O 'npm install' falhou." }

if ($Build) {
    Passo "Gerando o instalador (a primeira vez leva de 10 a 20 minutos)"
    & npm run tauri build
    if ($LASTEXITCODE -ne 0) { Falha "O build falhou. Copie as mensagens de erro acima." }
    $pasta = Join-Path $Destino "target\release\bundle\nsis"
    Ok "Instalador pronto em $pasta"
    if (Test-Path $pasta) { Invoke-Item $pasta }
}
else {
    Passo "Compilando e abrindo o FunkDown (a primeira vez leva de 5 a 10 minutos)"
    Write-Host "    A janela do app abre sozinha. Para encerrar, feche a janela ou aperte Ctrl+C aqui."
    & npm run tauri dev
    if ($LASTEXITCODE -ne 0) { Falha "O app não compilou ou fechou com erro. Copie as mensagens de erro acima." }
}
