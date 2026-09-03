# Cliché — publier une version

Ce document est la procédure de publication. Il est écrit pour être exécutable
par quelqu'un d'autre que son auteur, et pour rester valable le jour où la
décision sur les mises à jour automatiques s'inversera.

**Décision en vigueur (2 septembre 2026) : option A — aucun updater.**
L'arbitrage complet est dans [`UPDATES.md`](UPDATES.md). Ce document applique A,
et §6 ci-dessous dit exactement ce qui change le jour du passage à B.

---

## 1. Ce qu'une release doit contenir

| Élément | Pourquoi |
| --- | --- |
| L'installeur NSIS `.exe` | c'est ce qu'un humain télécharge |
| Sa somme **SHA-256**, publiée dans le corps de la release | contrôle d'intégrité — voir l'avertissement §4 |
| Les notes de version | ce qui a changé, en clair |
| Le tag `vX.Y.Z` | il doit correspondre aux trois numéros de version du dépôt |

---

## 2. Avant de publier — les contrôles, dans cet ordre

Aucun n'est facultatif. Le build en particulier n'est pas une étape optionnelle :
certaines erreurs n'apparaissent que là.

```powershell
work perso -NoCd; node scripts/check-version.mjs          # les 3 versions concordent
work perso -NoCd; pnpm typecheck                          # tsc --noEmit, strict
work perso -NoCd; cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
work perso -NoCd; cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
work perso -NoCd; pnpm test                               # check-version + check-contrast + cargo test
work perso -NoCd; pnpm tauri build                        # produit l'installeur NSIS
```

Puis **lancer l'installeur produit et l'application installée**. Un binaire
compilé n'est pas un binaire qui démarre : le 2 septembre 2026, `cargo build`
rendait 0 sur un exécutable que Windows refusait de lancer (manifeste transcodé
en Latin-1, `os error 14001`). Le build ne l'a pas vu. Seul le lancement l'a vu.

---

## 3. Le numéro de version — trois fichiers, un seul fait

La version vit dans **trois** fichiers, parce que trois chaînes d'outils
différentes lisent chacune la leur :

| Fichier | Qui le lit |
| --- | --- |
| `package.json` | pnpm, vite |
| `src-tauri/Cargo.toml` | cargo, et la version embarquée dans le binaire |
| `src-tauri/tauri.conf.json` | le bundler, l'installeur, et la chaîne que la page Aide affichera (lot 2) |

Trois copies d'un même fait, c'est exactement le motif que ce projet s'interdit
ailleurs. Il est toléré ici pour une raison précise et temporaire : **aucun
mécanisme de source unique n'a été vérifié** pour cette version de Tauri. La
documentation consultée le 2 septembre 2026 via Context7 ne répond pas sur le
comportement du champ `version` absent de `tauri.conf.json`, et une supposition
sur ce point se paierait le jour d'une release.

Donc les copies ne sont pas *crues*, elles sont **contrôlées** :
`scripts/check-version.mjs` échoue si elles divergent, et il est branché dans
`pnpm test` et dans la CI. Vérifié le 2 septembre 2026 dans les deux sens —
il rend 0 quand les trois concordent, 1 quand on en fait diverger une.

**À faire un jour** : vérifier empiriquement si retirer `version` de
`tauri.conf.json` fait bien retomber Tauri sur la version du `Cargo.toml`. Si
c'est le cas, deux fichiers suffisent et le script se simplifie. Tant que ce
n'est pas mesuré sur un binaire construit, on ne touche à rien.

---

## 4. La somme SHA-256 — ce qu'elle prouve, et ce qu'elle ne prouve pas

```powershell
work perso -NoCd; Get-FileHash .\src-tauri\target\release\bundle\nsis\*.exe -Algorithm SHA256
```

> **Elle protège d'un téléchargement corrompu. Elle ne protège pas d'un
> attaquant qui contrôle la page de release** — celui-là modifierait aussi la
> somme affichée à côté. C'est un contrôle d'**intégrité**, pas
> d'**authenticité**.

Cette phrase doit rester dans le corps de la release. Une somme publiée sans
elle laisse croire à une garantie qui n'existe pas.

L'authenticité, c'est la signature de code Authenticode — un sujet différent,
chiffré dans [`UPDATES.md §5`](UPDATES.md), et non résolu à ce jour.

---

## 5. Publier

```powershell
work perso -NoCd; git tag -a v0.1.0 -m "Cliche v0.1.0"
work perso -NoCd; git push origin v0.1.0
work perso -NoCd; gh release create v0.1.0 `
    ".\src-tauri\target\release\bundle\nsis\Cliche_0.1.0_x64-setup.exe" `
    --title "Cliché v0.1.0" --notes-file .\notes-v0.1.0.md
```

Le nom exact de l'installeur dépend de `productName` et de la version : le lire
dans la sortie de `pnpm tauri build` plutôt que le recopier d'ici.

**Retour arrière** : `gh release delete vX.Y.Z --yes` puis
`git push --delete origin vX.Y.Z`. À faire vite — une release téléchargée ne se
rattrape plus.

---

## 6. Le jour du passage à l'option B (updater)

Le déclencheur est écrit dans [`UPDATES.md §6`](UPDATES.md), et il mérite d'être
répété ici parce qu'il se manifeste **pendant** une release, pas avant :

> **L'updater ne peut mettre à jour que depuis une version qui le contient
> déjà.** Une copie installée sans updater devra être remplacée à la main, une
> dernière fois.

Conséquence concrète, et c'est le point qui décide du calendrier : le bon moment
pour activer B n'est pas « quand il y aura des utilisateurs », c'est **avant la
première copie distribuée à quelqu'un d'autre**. Chaque installation faite avant
est une réinstallation manuelle à organiser plus tard.

Aujourd'hui, une seule machine est concernée, et c'est celle qui compile : le
coût est nul. Il cesse de l'être à la première copie qui sort d'ici.

### Ce qui est déjà prêt, et ce qui resterait à faire

| Prêt aujourd'hui | Où |
| --- | --- |
| L'arbitrage écrit, chiffré, avec ses sources | `docs/UPDATES.md` |
| La liste exacte des 11 emplacements à modifier | `docs/UPDATES.md §7.2` |
| Cette procédure de publication | ce fichier |
| Le contrôle de cohérence des versions | `scripts/check-version.mjs`, branché en CI |
| Une CI qui construit l'installeur et le conserve en artefact | `.github/workflows/ci.yml` |
| `capabilities/` sans aucun plugin — l'ajout de l'updater y sera visible en une ligne de diff | `src-tauri/capabilities/` |

| Resterait à faire, le jour venu | Coût |
| --- | --- |
| Générer la paire de clés (`pnpm tauri signer generate`) | 30 s — **délibérément pas fait aujourd'hui**, voir ci-dessous |
| Déposer la clé privée dans le coffre SecretStore `DevContext` **et la sauvegarder** | sa perte est irréversible : `UPDATES.md §3.3` |
| Ajouter la clé publique dans `tauri.conf.json` (valeur littérale, jamais un chemin) | — |
| Les 10 autres emplacements de `UPDATES.md §7.2` | — |
| Rendre les artefacts accessibles anonymement | implique un dépôt public : `UPDATES.md §4.5` |

**Pourquoi la clé n'est pas générée aujourd'hui**, alors que c'est gratuit : une
clé privée qui existe est un actif à protéger pour toujours, et elle ne rend
aucun service tant qu'aucune clé publique n'est embarquée dans un binaire
distribué. Une clé générée un an avant son premier usage est une clé dont
personne ne sait plus où elle est. Le geste coûte trente secondes le jour venu ;
la garder coûte tous les jours d'ici là.

---

## 7. Signature de code Windows

Non résolue. Sans certificat Authenticode, SmartScreen avertit à chaque
installation. C'est supportable pour l'auteur du binaire, ça ne l'est pas pour
un client payant. Voir [`UPDATES.md §5`](UPDATES.md) : c'est une décision qui
arrive **en même temps** que l'updater, et c'est la plus urgente des deux.
