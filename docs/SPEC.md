# FunkDown v2: especificação

Reescrita do YTFunkDown (Python + Tkinter, só MP3 do YouTube) como app desktop para Windows e Linux, com suporte a Instagram, TikTok, Facebook e aos demais sites do yt-dlp. Este documento registra as decisões tomadas, o que está implementado e o que fica para depois.

## Decisões

| # | Tema | Decisão |
| --- | --- | --- |
| 1 | Público | Uso pessoal e de amigos. Repositório no GitHub por organização, sem venda. |
| 2, 5 | Stack | Tauri 2: núcleo em Rust, interface web. O yt-dlp continua sendo o motor, como binário separado. |
| 3 | Mídia | Automático por plataforma (YouTube → áudio, redes sociais → vídeo), com troca manual por download. |
| 4 | Login | Cookies do navegador ou arquivo `cookies.txt`. O app nunca recebe senha. |
| 6 | Sistemas | Windows e Linux. macOS fora (sem assinatura da Apple o app é bloqueado). |
| 7 | Homelab | Versão web fica para depois. O núcleo já é separado da interface para isso. |
| 8 | Atualização | yt-dlp se atualiza sozinho ao abrir (no máximo a cada 12 h). O atualizador do app existe no código, mas fica desligado (ver decisão 20). |
| 9 | Funções da v2.0 | Fila paralela, progresso e cancelar por item, pasta padrão com subpastas, capa e tags, histórico, qualidade, playlists do YouTube. |
| 10 | Interface | Svelte 5 + TypeScript + Vite. |
| 11 | Motor | Baixado na primeira execução dos releases oficiais, com SHA-256 obrigatório. No Linux usa `ffmpeg` e `deno` do sistema se existirem. |
| 12 | Repositório | Mesmo repositório; Python preservado em `legacy/`, na tag `v1-python` e na branch `legacy-python`. |
| 13 | Pacotes | NSIS no Windows, AppImage e `.deb` no Linux, sem assinatura de código paga. |
| 14 | Nome | FunkDown. |
| 15 | Idiomas | PT-BR e inglês, seguindo o sistema por padrão. |
| 16 | Arquivos | `Artista - Título` com acentos; `@usuario - descrição (id)` nas redes sociais; 3 simultâneos; subpastas por plataforma. |
| 17 | Engenharia | CI em Windows e Linux, testes no núcleo, clippy, `cargo audit`, `npm audit`, Dependabot. Sem testes que dependam de sites reais. |
| 18 | Visual | Ícone novo (equalizador em forma de seta para baixo), tema claro e escuro seguindo o sistema. |
| 19 | Privacidade | Zero telemetria. Cookies só lidos pelo yt-dlp no download. Histórico local, com limpar e desligar. |
| 20 | Distribuição | Sem instaladores publicados: cada pessoa compila a própria cópia (`setup-windows.ps1` no Windows). O workflow de release fica guardado em `docs/release.yml.example`. Decidido em 16/11/2026 para não distribuir binários. |

Um ponto que surgiu na implementação: desde o fim de 2025 o yt-dlp precisa de um runtime JavaScript externo para o YouTube, e o recomendado é o Deno. Por isso o motor tem três peças (yt-dlp, FFmpeg e Deno), todas verificadas por SHA-256.

## Arquitetura

```
Interface (Svelte)  ──invoke──▶  Casca Tauri  ──▶  funkdown-core  ──processo──▶  yt-dlp ─▶ ffmpeg / deno
        ▲                            │                  │
        └────────── eventos ─────────┘◀─── Event ───────┘
```

- **`funkdown-core`** não conhece o Tauri. Recebe um `Emitter` (uma função) e publica `Event::Job` a cada mudança. Qualquer interface serve: a casca Tauri, a CLI de exemplo (`examples/fdl.rs`) ou um futuro servidor web.
- **Um processo yt-dlp por item da fila.** O núcleo passa templates (`--progress-template`, `--print`) que fazem o yt-dlp imprimir linhas com prefixos próprios (`FDPROG|`, `FDMETA|`, `FDFILE|`), fáceis de separar do resto da saída.
- **Nomes e tags são resolvidos pelo próprio yt-dlp** em uma única execução, com `--parse-metadata`: se o título já é "Artista - Título", usa; senão usa artista e faixa do YouTube Music; senão o canal.
- **Arquivos parciais** ficam em uma pasta temporária por item (`-P temp:`), apagada ao terminar ou cancelar. A pasta de downloads nunca fica com `.part` órfão.
- **Estado em disco**: `settings.json`, `history.json` e a pasta `bin/` do motor, nas pastas de dados do app. Gravação atômica (arquivo temporário + rename).

## Segurança

- Links: só `http`/`https` com host, passados depois de `--` e sem shell.
- Navegador dos cookies: lista fechada; valor fora dela nunca chega à linha de comando.
- Recorte: formato validado (`SS`, `MM:SS`, `HH:MM:SS`) antes de virar argumento.
- Motor: HTTPS obrigatório, hash conferido antes de instalar, extração só dos executáveis esperados pelo nome final (sem caminhos vindos do zip).
- Interface: CSP restrita; sem permissão de sistema de arquivos; abrir arquivo aceita só o id de um download conhecido.

## Implementado na 2.0

Tudo da decisão 9, mais:

- Recorte de trecho (`--download-sections`).
- Qualquer site suportado pelo yt-dlp, não só os quatro principais.
- Capa recortada em quadrado.
- SponsorBlock opcional para cortar trechos sem música.
- Mensagens de erro traduzidas por causa provável, com atalho para configurar o login.
- Colar em qualquer lugar da janela e arrastar links.
- Notificação quando um download termina com a janela em segundo plano.
- Histórico com busca e "baixar de novo".
- Atualização manual do motor nos Ajustes e exibição das versões.
- CLI de exemplo sobre o núcleo.

## Como foi verificado

- 24 testes unitários no núcleo (plataformas, argumentos, progresso, erros, checksums, ajustes, histórico).
- Fluxo completo no Linux com o app real: instalação do motor com SHA-256, download de áudio e de vídeo, recorte, nomes, tags, capa e histórico, usando um servidor HTTP local como origem.
- Compilação, execução e geração do instalador no Windows confirmadas pelo autor em 15/11/2026.
- **Não verificado por testes automatizados:** downloads reais de YouTube, Instagram, TikTok e Facebook.

## Próximos passos

1. Testar um link de cada plataforma, cancelamento no meio e login por cookies.
2. Publicar o código no GitHub (sem releases) e o post no LinkedIn.
3. **Versão web no homelab**: servidor HTTP sobre o `funkdown-core`, a mesma interface Svelte servida pelo navegador, em Docker.
4. Detecção de link copiado na área de transferência (opcional, desligado por padrão).
5. Fila persistente entre execuções.
6. Normalização de volume (loudnorm) como opção para áudio.
