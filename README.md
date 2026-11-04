# subscription-tv

Assista sem parar os vídeos recentes dos canais em que você é inscrito no YouTube. Você escolhe o período (ex.: últimos 7 dias) e a ordem, e o player passa sozinho para o próximo vídeo quando um termina.

Não precisa de login nem de chave de API: a lista de inscrições vem do Google Takeout e os vídeos vêm dos feeds RSS públicos de cada canal. No fim, tudo vira **um único binário**.

- **Backend:** Rust ([axum](https://github.com/tokio-rs/axum)). Lê o CSV, busca os feeds em paralelo e serve o site, que vai embutido no binário via [rust-embed](https://github.com/pyrossh/rust-embed).
- **Frontend:** Angular com [`@angular/youtube-player`](https://github.com/angular/components/tree/main/src/youtube-player).

## 1. Exporte suas inscrições

1. Acesse [takeout.google.com](https://takeout.google.com) e clique em **Desmarcar tudo**.
2. Marque só **YouTube e YouTube Music**. Em **Todos os dados do YouTube incluídos**, deixe marcado apenas **inscrições**.
3. Exporte, baixe o zip e procure o `subscriptions.csv` dentro da pasta do YouTube.

O arquivo fica fora do binário. Quando você se inscrever em canais novos, basta exportar de novo e substituir o CSV, sem recompilar.

## 2. Compile

Requisitos: [Rust](https://rustup.rs) e [Node.js](https://nodejs.org) (só para compilar; o binário final não precisa de nenhum dos dois).

```sh
cd frontend && npm install && npx ng build   # gera o site
cd ../backend && cargo build --release       # embute o site no binário
```

O binário fica em `backend/target/release/backend`. Copie para onde quiser e renomeie se preferir.

> Rode sempre o `ng build` antes do `cargo build --release`. Se esquecer, o binário sai com a versão anterior do site.

## 3. Rode

```sh
./backend                          # usa o subscriptions.csv da pasta atual
./backend caminho/para/subscriptions.csv
```

Abra **http://localhost:3000**, escolha os dias e a ordem e clique em **Carregar**.

### No celular

O servidor escuta na rede local. Com o celular no mesmo Wi-Fi, abra `http://<IP-do-PC>:3000`. No Linux, o IP aparece com `ip -4 addr`.

Isso também quer dizer que qualquer pessoa na mesma rede consegue abrir a página. Em casa não tem problema, mas evite deixar rodando em Wi-Fi público.

## Desenvolvimento

```sh
cd backend && cargo run -- ../subscriptions.csv   # API em :3000
cd frontend && npx ng serve                       # site em :4200 com hot reload; /api vai pro :3000
```

Testes do backend: `cd backend && cargo test`.

## Limitações

- O RSS do YouTube traz só os **15 vídeos mais recentes** de cada canal. Canais que publicam muito, como os de notícia, podem vir incompletos em períodos longos.
- Shorts e lives entram na fila normalmente.
- Canais apagados ou encerrados retornam 404 e são ignorados. O servidor só registra no log.
- No celular, o navegador pode bloquear o play automático do próximo vídeo.

## Licença

[MIT](LICENSE)
