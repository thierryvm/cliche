# Cliché — mise à jour automatique : la décision

> Document de **décision**, en français. Établi le **2 septembre 2026**.
> Aucune ligne de code n'a été écrite pour le produire, aucun fichier du projet
> n'a été modifié. Toute date est absolue.
>
> Ce document tranche **si** Cliché doit se mettre à jour tout seul, et à quel
> prix. Il ne configure rien. La section 7 liste ce qu'il faudrait toucher —
> sans le toucher.

---

## 0. La contradiction, d'abord

La contrainte posée, mot pour mot :

> « Tout reste sur ma machine. Pas de compte, pas de lien de partage, rien qui sort. »

La demande posée aujourd'hui : la mise à jour automatique.

**Les deux ne peuvent pas être vraies en même temps.** Un updater ne peut pas
savoir qu'une version existe sans le demander à quelqu'un. Il n'existe aucune
implémentation, aucune option de configuration, aucun réglage de discrétion qui
supprime cette requête : la requête *est* la fonctionnalité. Ajouter un updater,
c'est retirer « rien qui sort » de la liste des propriétés de l'application et
le remplacer par « voici exactement ce qui sort, et quand ».

Ce n'est pas une nuance rhétorique, et voici la preuve la plus courte que je
puisse en donner — mesurée, pas déduite. `tauri-plugin-updater` 2.11.0 déclare
parmi ses dépendances directes :

```
reqwest ^0.13         <- un client HTTP complet
minisign-verify ^0.2  <- la vérification de signature
tauri ^2.10           <- compatible avec le tauri 2.11.5 du projet
```

*(source : API crates.io, `/api/v1/crates/tauri-plugin-updater/2.11.0/dependencies`,
interrogée le 2 septembre 2026)*

Or `README.md` du projet affirme aujourd'hui, ligne 6 :

> *« There is no HTTP client in the dependency tree »*

Cette phrase **devient fausse** à la ligne où le plugin entre dans
`src-tauri/Cargo.toml`. Elle ne devient pas « moins vraie », ou « vraie avec une
réserve » : fausse. Elle devra être retirée, pas nuancée. C'est le coût le plus
visible de la décision, et c'est aussi le plus honnête : il se mesure en une
ligne de `README`.

### Un piège à écarter tout de suite : la CSP ne protège pas de ça

`src-tauri/tauri.conf.json` restreint aujourd'hui `connect-src` à
`ipc: http://ipc.localhost` (ligne 25). On pourrait croire que cette CSP
empêcherait toute sortie réseau, updater compris. **Elle ne l'empêcherait pas.**

La CSP est appliquée par WebView2 au document affiché. La requête de l'updater
part de `reqwest`, un client HTTP **Rust**, dans le processus natif — hors du
webview, hors de sa pile réseau, hors de sa CSP. La CSP resterait verte et
inchangée pendant que l'application appellerait un serveur. Elle protège de ce
que la page pourrait faire ; elle ne dit rien de ce que le binaire fait.

*(Fondement : `reqwest` figure dans les dépendances Rust du plugin — mesuré.
Le fait que la CSP d'un document ne gouverne pas un client HTTP natif du même
processus est une conséquence structurelle, pas une mesure que j'ai faite ici ;
voir §8.)*

---

## 1. Ce qui sort de la machine, exactement

C'est l'information qu'on ne donne jamais. La voici, requête par requête.

### 1.1 La vérification, cas d'un manifeste **statique**

L'application fait un `GET` sur une URL fixe, par exemple
`https://github.com/thierryvm/cliche/releases/latest/download/latest.json`.

**Ce que le serveur (GitHub) enregistre :**

| Donnée | Détail |
| --- | --- |
| **Adresse IP source** | Celle de la connexion de Thierry. Sur une ligne fixe belge, elle est stable pendant des mois : c'est un identifiant, pas une donnée volatile. Elle donne l'opérateur et une géolocalisation à la ville. |
| **Horodatage** | À la seconde, côté serveur. |
| **Chemin demandé** | `/thierryvm/cliche/releases/latest/download/latest.json` — donc le **nom du produit** et le fait qu'il s'agit d'une vérification de mise à jour. |
| **En-tête `User-Agent`** | Celui de `reqwest` par défaut, sauf si l'application le remplace. Il identifie la bibliothèque, donc indirectement la nature du client. |
| **Empreinte TLS** | Le `ClientHello` (suites, extensions, ALPN) est propre à la pile TLS employée. Il ne nomme personne, mais il distingue. |

**Ce que le résolveur DNS voit** (opérateur, ou Cloudflare/Google si configuré
ainsi) : le nom d'hôte interrogé, depuis l'IP de la machine, à cet instant. Le
DNS voit donc la vérification même si la requête HTTPS, elle, est chiffrée.

**Ce que le réseau intermédiaire voit** : le nom d'hôte en clair via le SNI TLS
(sauf ECH, non acquis), la taille et l'instant des paquets. Pas le contenu.

**Ce qui NE sort PAS dans ce cas** : la version installée. Avec un manifeste
statique, le client télécharge le manifeste entier et compare les numéros
**localement**. C'est une différence réelle, et elle est en faveur du statique.

### 1.2 La vérification, cas d'un endpoint **dynamique**

La documentation Tauri (page `v2.tauri.app/plugin/updater/`, consultée le
2 septembre 2026) décrit trois variables interpolables dans l'URL d'endpoint :
`{{current_version}}`, `{{target}}`, `{{arch}}`. Le serveur répond `204 No Content`
quand il n'y a rien, `200` avec un JSON sinon.

Conséquence directe : **la version installée, le système et l'architecture CPU
partent dans le chemin de l'URL**, donc dans le journal d'accès du serveur, en
clair côté serveur, et dans tout cache ou proxy qui journalise les URL.

C'est le mode qui en dit le plus. Il n'apporte rien ici : il sert à faire des
mises à jour par paliers ou par cohortes, ce qu'un projet à un utilisateur ne
fera jamais.

### 1.3 Le téléchargement, si une version est disponible

Un second `GET`, vers l'URL du binaire (quelques dizaines de Mo pour un NSIS
Tauri). Mêmes données que ci-dessus, plus : le **volume transféré**, qui trahit
qu'il s'agit d'une installation et non d'une vérification à vide. Sur GitHub, le
compteur de téléchargements par artefact est exposé par l'API publique (§8 :
non mesuré ici, à confirmer avant de s'en inquiéter).

### 1.4 Ce que la RÉPÉTITION construit — le vrai sujet

Une requête isolée ne dit presque rien. **Une requête à chaque démarrage, pendant
deux ans, dit beaucoup**, et c'est ce que personne n'énonce :

- **Les heures de travail.** Le premier lancement de la journée marque le début
  de journée, le dernier la fin. Sur des semaines, ça donne un rythme.
- **Les jours de congé, les week-ends, les vacances, les maladies.** Ce sont les
  trous. Un trou de dix jours en juillet se lit sans effort.
- **Les changements de lieu.** Une IP qui change signale un déplacement, un
  hôtel, un déménagement, un autre réseau.
- **Le lien entre tout ça et une identité.** Le dépôt s'appelle
  `thierryvm/cliche`. Le chemin de la requête porte le nom du compte GitHub.
  L'IP est celle du domicile. Ce n'est pas un profil anonyme : c'est un journal
  d'activité nominatif, hébergé chez un tiers, que le tiers conserve selon sa
  politique et non selon la vôtre.

Et il faut le dire jusqu'au bout : sur GitHub Pages, la documentation de GitHub
énonce elle-même — page *What is GitHub Pages*, consultée le 2 septembre 2026 —
que *« the visitor's IP address is logged and stored for security purposes,
regardless of whether the visitor has signed into GitHub or not »*. Ce n'est pas
une supposition sur leurs pratiques : c'est leur propre phrase.

**Avec un seul utilisateur, ce journal ne décrit qu'une personne : Thierry.**
C'est une différence de nature avec un logiciel à 100 000 utilisateurs, où
chacun se cache dans le nombre. Ici, il n'y a pas de nombre.

### 1.5 L'ordre de grandeur, pour ne pas dramatiser non plus

Ce même navigateur, ce même Windows, cet éditeur, `pnpm`, `cargo`, Discord —
tous parlent déjà à leurs serveurs depuis cette machine, plus souvent et en
disant davantage. L'updater de Cliché n'ouvrirait pas une brèche dans un mur
étanche ; il ajouterait un flux de plus à un réseau déjà bavard.

Ce qu'il détruirait, ce n'est pas la confidentialité de Thierry. C'est **la
propriété que le projet revendique** : « cette application-ci ne parle à
personne », phrase aujourd'hui vraie, vérifiable en lisant `Cargo.toml`, et
qui est une partie du produit. C'est ça qu'on met en jeu — et c'est une décision
de produit, pas une décision de vie privée.

---

## 2. Trois options, honnêtement comparées

### Option A — aucun updater

L'application ne contient aucun code réseau. Les versions vivent sur une page ;
on télécharge et on installe à la main.

| | |
| --- | --- |
| **Ce qui sort** | **Rien.** Zéro requête depuis l'application, jamais. Le téléchargement est une action du navigateur, décidée et vue par l'utilisateur. |
| **Coût de construction** | Quasi nul. Une page de versions, un installeur publié, une somme SHA-256 à côté. Rien dans le code. |
| **Coût de maintenance** | Publier une version = produire l'installeur et l'attacher à une release. Pas de manifeste à tenir à jour, pas de clé à garder, pas de format à respecter. |
| **Ce que ça ferme** | **Toute possibilité de pousser un correctif.** Si un défaut de sécurité apparaît — et le lot 4 en héberge un candidat sérieux, le flou réversible — il n'existe aucun canal pour l'atteindre. Il faut que l'utilisateur pense à regarder. Aujourd'hui l'utilisateur, c'est Thierry, et il saura. Demain, non. |
| **Ce que ça ferme aussi** | Les installations produites sous A **n'embarquent aucune clé publique**. Le jour où B est adopté, chaque installation existante devra être remplacée **une fois, à la main**, avant de pouvoir se mettre à jour. Coût borné, mais réel (voir §3.4). |

### Option B — updater **opt-in**, déclenché par un clic

Le plugin est présent. **Aucune requête ne part au démarrage.** Un bouton
« Vérifier les mises à jour » existe dans l'application ; tant qu'il n'est pas
pressé, la pile réseau ne sert jamais.

| | |
| --- | --- |
| **Ce qui sort** | Exactement ce que décrit §1.1 — **et seulement quand l'utilisateur l'a demandé**. Une requête par clic, zéro sinon. L'utilisateur sait qu'elle part, parce qu'il vient de la déclencher. C'est la différence morale entre les deux options réseau, et elle est entière. |
| **Coût de construction** | Le plugin, la clé de signature, la configuration `pubkey` + `endpoints`, la permission dans `capabilities/`, le bouton et ses **cinq états** — inactif, en cours, à jour, mise à jour trouvée, échec réseau. C'est le dernier état qui coûte : une application « sans réseau » doit échouer proprement quand le réseau manque, sans alarmer. |
| **Coût de maintenance** | **Le vrai coût est là.** Chaque version doit être signée avec la même clé, produire un manifeste au bon format, et être publiée à la même URL. Une release faite à la main un soir de fatigue, sans signature, et l'updater rejette silencieusement la mise à jour chez tout le monde. La discipline devient permanente. |
| **Ce que ça ferme** | La phrase « rien ne sort » — remplacée par « rien ne sort sans votre clic ». Défendable, mais différent. Et l'engagement sur l'hébergement (§4) devient irréversible : changer l'URL du manifeste après coup casse les installations qui portent l'ancienne. |

### Option C — vérification automatique au démarrage

| | |
| --- | --- |
| **Ce qui sort** | §1.1 **à chaque lancement**, sans que l'utilisateur le décide ni le voie. Donc §1.4 en entier : le motif d'usage, construit passivement, sur la durée. |
| **Coût de construction** | Marginalement inférieur à B : pas de bouton, pas d'états d'interface à soigner. C'est la seule chose qu'elle a pour elle. |
| **Coût de maintenance** | Identique à B, plus deux dettes : gérer le cas où le réseau est absent au démarrage **sans ralentir le lancement** (or le projet vise une médiane sous 150 ms au lot 1 — une requête réseau synchrone au démarrage est exactement ce qu'il ne faut pas faire), et gérer l'installeur qui **ferme l'application** pendant qu'elle travaille. |
| **Ce que ça ferme** | Le contrôle de l'utilisateur sur ses propres sorties réseau. Sur un outil qui **lit l'écran** — donc potentiellement des mots de passe, des messages, des documents clients — une sortie réseau silencieuse au démarrage est la pire propriété qu'on puisse lui donner. Pas parce qu'elle enverrait des images : elle n'en enverrait pas. Parce qu'elle rend l'affirmation « il n'envoie rien » **invérifiable pour qui n'a pas lu le code**. |

**C est écartée dès maintenant**, et pas seulement au nom de la vie privée : sur
un outil de capture d'écran, une requête sortante non sollicitée est un défaut
de conception, pas un réglage.

---

## 3. Les clés de signature

### 3.1 Le mécanisme, mesuré

`tauri-plugin-updater` 2.11.0 dépend de **`minisign-verify ^0.2`**
*(API crates.io, 2 septembre 2026)*. Le mécanisme est donc **minisign** : une
signature Ed25519, sans autorité de certification, sans chaîne de confiance,
sans révocation. Une clé publique, une clé privée, et rien d'autre.

Côté outillage, vérifié **sur cette machine** le 2 septembre 2026 avec le
`@tauri-apps/cli` 2.11.4 déjà installé (`pnpm tauri signer --help`) :

```
Commands:
  sign      Sign a file
  generate  Generate a new signing key to sign files

generate options:
  -p, --password <PASSWORD>      Set private key password when signing
  -w, --write-keys <WRITE_KEYS>  Write private key to a file
  -f, --force                    Overwrite private key even if it exists
      --ci                       Skip prompting for values [env: CI=]
```

La production des artefacts se fait par la clé de bundle
**`bundle.createUpdaterArtifacts`**, dont le schéma officiel
(`https://schema.tauri.app/config/2`, téléchargé le 2 septembre 2026) donne la
description exacte : *« Produce updaters and their signatures or not »*,
**défaut `false`**.

D'après la documentation Tauri (même page, même date) : la clé publique se place
dans la configuration sous `pubkey` et *« cannot be a file path »* — elle est
donc **littéralement écrite dans `tauri.conf.json`, et compilée dans le binaire**.
La clé privée passe par les variables d'environnement
`TAURI_SIGNING_PRIVATE_KEY` et `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, la page
précisant que les fichiers `.env` ne fonctionnent pas.

### 3.2 Où vivrait la clé privée

**Pas dans le dépôt.** `.gitignore` couvre déjà `*.key` (ligne 25) et `.tauri/`
(ligne 26) depuis aujourd'hui, avec le commentaire qui explique pourquoi : une
clé poussée une fois est compromise pour toujours, et réécrire l'historique ne
la dépublie pas — GitHub sert les objets devenus inaccessibles par leur SHA.

**Mais il faut voir ce que ce `.gitignore` ne couvre pas**, sans quoi il donne
une fausse assurance :

1. `tauri signer generate` **sans `-w`** affiche la clé privée sur la sortie
   standard. Elle atterrit alors dans l'historique du terminal, dans un journal
   de session, et — si un agent a lancé la commande — dans une transcription de
   conversation. `.gitignore` n'y peut rien.
2. `-w ~/.tauri/cliche.key` écrit **hors du dépôt**, dans le profil utilisateur.
   `.gitignore` n'y peut rien non plus. Le fichier est alors emporté par toute
   sauvegarde du profil, toute synchronisation cloud du dossier utilisateur.
3. En CI, la clé vit dans un secret GitHub Actions. `.gitignore` n'y peut rien.

**La règle qui couvre les trois** : la clé privée va dans le coffre SecretStore
`DevContext`, sous une clé du type `devctx/perso/tauri-signing-key`, à côté de
`github-token` et `vercel-token`. Elle est exportée en variable d'environnement
par `work perso` au moment de la release, et elle ne touche jamais le disque en
clair. C'est le mécanisme déjà en place sur cette machine ; il n'y a rien à
inventer.

### 3.3 Si la clé privée est PERDUE — la réponse brutale

Elle l'est, disons, dans un formatage, ou parce que le coffre n'a pas été
sauvegardé.

1. **Aucune nouvelle mise à jour ne peut plus être signée.** Il n'y a pas de
   récupération : minisign n'a pas d'autorité, pas de clé de secours, pas de
   séquestre. La clé perdue est perdue.
2. On peut évidemment générer une **nouvelle** paire. Mais les installations
   déjà déployées portent l'**ancienne clé publique gravée dans leur binaire**
   (§3.1 : `pubkey` est compilé, pas lu à l'exécution). Elles vérifieront tout
   nouveau manifeste avec l'ancienne clé, et **le rejetteront**.
3. **Oui : ces installations deviennent orphelines.** Définitivement, et en
   silence — l'utilisateur verra « à jour » ou une erreur de vérification, pas
   « votre canal de mise à jour est mort ». Le seul moyen de les récupérer est
   que quelqu'un aille sur chaque machine réinstaller à la main une version
   portant la nouvelle clé publique.
4. Il n'existe **aucun mécanisme de rotation** : pas de signature croisée
   ancienne-clé-vers-nouvelle, pas de liste de clés acceptées, pas de CRL. La
   clé publique du jour 1 est un engagement pour toute la durée de vie des
   binaires du jour 1.

Aujourd'hui, avec une seule installation, sur cette machine, ce scénario coûte
**une réinstallation manuelle**. C'est trivial. C'est précisément pourquoi il
faut le dire maintenant : ce n'est pas la perte de la clé qui est chère, c'est
la perte de la clé **quand il y a des utilisateurs**. Et le moment où on cesse
de pouvoir les compter sur une main n'est jamais annoncé.

### 3.4 Si la clé privée FUITE

C'est le scénario grave, et il est asymétrique du précédent.

Détenir la clé ne suffit pas à attaquer : il faut aussi **contrôler ce que
l'application va chercher** — le dépôt GitHub, le compte qui le tient, la
résolution DNS, ou le réseau de la victime. Mais il faut voir où vivent ces deux
capacités : **la clé de signature et le jeton GitHub sont dans le même coffre,
sur la même machine, sous la même session.** Une compromission de ce poste les
donne toutes les deux. Le « il faut deux choses » se réduit alors à « il faut
une chose ».

Et si les deux tombent :

- L'attaquant publie une release signée. Chaque installation la vérifie
  **avec succès** — la signature est authentique — et l'installe.
- Le code s'exécute **avec les privilèges de l'utilisateur**, sur une machine
  où l'application a déjà, par nature, le droit de **lire l'écran entier**.
- **Sous l'option C, tout cela est silencieux et automatique.** Sous B, il faut
  attendre que l'utilisateur clique. C'est mince, mais ce n'est pas rien.
- **Il n'y a aucune révocation.** On ne peut pas invalider la clé volée : les
  binaires déployés lui font confiance et ne consultent rien d'autre. Le seul
  remède est de publier une version portant une nouvelle clé publique et
  d'obtenir que **chaque** utilisateur la réinstalle à la main — c'est-à-dire
  exactement la procédure du §3.3, mais avec un attaquant qui, entre-temps, garde
  la main sur toutes les installations non migrées.

**En clair : la clé de signature d'un updater est une clé d'exécution de code à
distance sur toutes les machines où l'application est installée.** Elle mérite le
même soin qu'une clé SSH de production, et pas moins.

---

## 4. Où vivent le manifeste et les binaires — et ce que ça décide du dépôt

État mesuré du dépôt le 2 septembre 2026 :
`git log` répond *« your current branch 'main' does not have any commits yet »*,
`git remote -v` ne rend rien. **Rien n'est poussé, aucun dépôt GitHub n'existe.**
Tout ce qui suit est donc encore ouvert — et cessera de l'être au premier `git push`.

### 4.1 GitHub Releases, dépôt public

Mesuré le 2 septembre 2026, en anonyme, sans jeton :

```
GET https://api.github.com/repos/thierryvm/devcontext/releases    -> HTTP 200   (dépôt public)
GET https://api.github.com/repos/thierryvm/claude-config/releases -> HTTP 404   (dépôt privé)
```

Le manifeste et l'installeur sont accessibles sans authentification, à une URL
stable, gratuitement, avec la bande passante de GitHub. **C'est la seule option
gratuite qui fonctionne réellement.**

### 4.2 GitHub Releases, dépôt privé — impossible en pratique

Le `404` ci-dessus n'est pas un refus poli : GitHub renvoie `404` et non `403`
précisément pour ne pas révéler l'existence du dépôt. Un updater anonyme n'a
aucun moyen de lire ce manifeste.

Le seul contournement serait d'**embarquer un jeton d'accès dans l'application
distribuée**. C'est-à-dire livrer une clé d'accès à ses dépôts privés dans un
binaire que n'importe qui peut ouvrir avec `strings`. **Ce n'est pas une option
à comparer, c'est une faute.** Elle est écartée sans discussion.

### 4.3 GitHub Pages

La documentation GitHub (*Changing the visibility of your GitHub Pages site*,
consultée le 2 septembre 2026) est explicite : *« To publish a GitHub Pages site
privately, your organization must use GitHub Enterprise Cloud. »* Pour un compte
personnel, **le site est public**. Et §1.4 rappelle que GitHub journalise l'IP
des visiteurs, de son propre aveu.

### 4.4 Un hébergement statique tiers (Vercel, Netlify, un VPS)

Techniquement possible, et ça découple le manifeste du dépôt. Mais ça ajoute un
compte, une facture ou un renouvellement, un domaine à ne pas laisser expirer —
**et si ce domaine expire, l'URL de mise à jour de tous les binaires déployés
devient la propriété du premier qui la rachète.** Combiné à une clé qui aurait
fuité, c'est le scénario complet du §3.4, offert. Pour un projet solo, c'est plus
de surface, pas moins.

### 4.5 La conséquence, écrite en clair

> **Choisir un updater (B ou C), c'est choisir de rendre le dépôt public, ou au
> minimum de publier des artefacts accessibles anonymement à une URL stable.**
> Il n'existe pas de troisième voie gratuite.

Ce n'est pas nécessairement un mal : le projet vise aussi le portfolio, donc il
sera public de toute façon. Et pour un outil qui lit l'écran, un code source
auditable est un **argument de confiance**, pas une concession.

Mais ça doit être **décidé**, pas subi. Rendre public un dépôt, c'est publier
l'historique complet, les messages de commit, les chemins de fichiers, les
rythmes de travail, et l'adresse e-mail de commit. Ce sont des données. Elles ne
se retirent pas après coup : GitHub sert les objets par SHA, y compris ceux
qu'un `force-push` a rendus inaccessibles — c'est déjà le raisonnement écrit dans
`.gitignore`, il vaut aussi pour le reste.

---

## 5. Signature de code Windows et SmartScreen — un sujet DIFFÉRENT

C'est la confusion la plus fréquente, et elle coûte cher dans les deux sens :
soit on paie un certificat en croyant sécuriser l'updater, soit on signe
l'updater en croyant faire taire Windows. **Les deux signatures n'ont ni le même
objet, ni le même vérificateur, ni le même moment.**

| | Signature updater (minisign) | Signature de code (Authenticode) |
| --- | --- | --- |
| **Répond à** | « ce fichier vient bien du détenteur de la clé » | « cet exécutable vient d'une identité vérifiée par une AC » |
| **Vérifiée par** | l'application Cliché elle-même | **Windows**, plus SmartScreen |
| **Quand** | au moment de la mise à jour | au téléchargement et à chaque lancement |
| **Coût** | 0 € | plusieurs centaines d'euros par an (§5.2) |
| **Autorité** | aucune. Une paire de clés, point | une AC publiquement reconnue |

La preuve que ce sont deux mondes : dans le **même** `tauri.conf.json`, elles
vivent dans deux endroits sans rapport. Vérifié dans le schéma officiel
(téléchargé le 2 septembre 2026) — `bundle.createUpdaterArtifacts` d'un côté, et
de l'autre, sous `bundle.windows` : `certificateThumbprint`
(*« Specifies the SHA1 hash of the signing certificate »*), `digestAlgorithm`,
`timestampUrl` (*« Server to use during timestamping »*), `signCommand`, `tsp`.

**Signer l'updater ne fait pas disparaître un seul avertissement Windows.**

### 5.1 Ce qui se passe concrètement avec un installeur non signé

Documentation Microsoft, *Microsoft Defender SmartScreen overview*
(`learn.microsoft.com`, champ `ms.date` : **23 avril 2026**), phrases exactes :

> *« Checking downloaded files against a list of files that are well known and
> downloaded frequently. If the file isn't on that list, Microsoft Defender
> SmartScreen shows a warning, advising caution. »*

> *« If a URL, a file, an app, or a certificate has an established reputation,
> users don't see any warnings. If there's no reputation, the item is marked as
> a higher risk and presents a warning to the user. »*

Traduit en conséquences pour Cliché :

- Le `Cliche_0.1.0_x64-setup.exe` téléchargé n'est **connu de personne**. Il
  déclenche donc l'avertissement. Le bouton pour continuer existe, mais il n'est
  pas au premier plan.
- **Le critère est le volume de téléchargements**, pas la simple présence d'une
  signature. C'est écrit dans le passage ci-dessus.
- L'application ne sera jamais téléchargée en volume. Elle n'acquerra donc
  jamais de réputation par ce chemin.

### 5.2 Ce que coûte un certificat

Prix relevés le **2 septembre 2026** sur la page commerciale de SSL.com,
autorité de certification (donc source directe pour son propre tarif) :

- EV code signing : **349,00 $/an** pour un an ; 149,00 $/an sur cinq ans.
- Jeton matériel YubiKey : **+379,00 $**.
- HSM en nuage, frais d'attestation : 500 $ (Google, Azure) à 1 500 $ (AWS).

Soit, en réaliste, **de l'ordre de 700 $ la première année** pour la voie EV avec
jeton. Des comparateurs commerciaux (recherche web du 2 septembre 2026)
annoncent de l'OV à partir de ~219 $/an ; ce sont des pages de vente, à traiter
comme telles.

Deux points structurels rapportés par ces mêmes sources, **non confirmés à la
source primaire** (voir §8) :

- Depuis **juin 2023**, les exigences de base du CA/Browser Forum imposeraient
  que la clé privée soit générée et conservée sur du matériel certifié
  FIPS 140-2 niveau 2 ou Common Criteria EAL 4+. Fini le `.pfx` sur le disque :
  d'où le jeton à 379 $.
- SSL.com écrit sur sa propre page : *« Since March 2024, Microsoft's Trusted
  Root Program update removed EV's distinct SmartScreen status — EV and OV
  certificates now build SmartScreen reputation equally through download volume.
  EV certificates no longer receive instant SmartScreen bypass. »*

### 5.3 Pourquoi c'est probablement hors sujet ici

Si cette dernière citation est exacte — et elle vient d'une AC qui n'a aucun
intérêt commercial à minimiser la valeur de son produit EV, ce qui la rend
crédible —, alors **payer 700 $ n'achèterait pas le silence de SmartScreen**.
Ça achèterait le droit de commencer à construire une réputation, laquelle se
construit par le volume de téléchargements, lequel sera de **un**.

On paierait 700 $/an pour, dans le meilleur des cas, voir l'avertissement
persister plusieurs mois.

Et surtout : **Thierry est le seul à installer Cliché, et c'est lui qui l'a
compilée.** Il ne va pas s'authentifier à lui-même la provenance d'un binaire
qu'il vient de produire. L'avertissement, il le voit une fois par version, il
clique, c'est fini.

**Le seuil où ça change** — et il faut le nommer, pas le laisser flou : le jour
où l'installeur est donné à quelqu'un qui ne l'a pas compilé. Ce jour-là,
« éditeur inconnu » sur un outil qui capture l'écran est la pire phrase possible
au pire moment. Ce n'est pas un problème technique, c'est un problème de
crédibilité — et il apparaîtra **avant** le besoin d'un updater, pas après.

---

## 6. Recommandation

> ### Option A maintenant. Option B au premier utilisateur qui n'est pas Thierry. Jamais C.

**Pourquoi A, argumenté et non par paresse.**

Un updater sert à porter du code sur des machines qu'on ne touche pas. Thierry
touche la seule machine concernée, tous les jours, et c'est lui qui compile.
Construire aujourd'hui un canal de mise à jour, c'est écrire un pont vers une
rive vide : le coût est immédiat et permanent — une clé à garder pour toujours
(§3.3), une discipline de release à ne jamais rompre, `reqwest` dans l'arbre de
dépendances, une phrase du `README` à retirer, un hébergement public engagé pour
la vie des binaires (§4.5) — pendant que le bénéfice est nul et le restera tant
que le nombre d'installations distantes vaut zéro.

**C'est de la dette déguisée en fonctionnalité, et je le dis nettement** : le
signe qui ne trompe pas est qu'on ne peut désigner aucun utilisateur qui en
bénéficierait. Une fonctionnalité qui ne peut pas nommer son bénéficiaire n'en
est pas une.

Il y a un contre-argument sérieux, et il faut y répondre plutôt que l'ignorer :
*le projet vise le portfolio, et un updater fait sérieux*. Réponse : ce qui fait
sérieux, c'est **ce document**. Un updater générique branché en trois lignes ne
distingue personne — n'importe quel modèle le génère. Un arbitrage écrit, chiffré,
qui explique ce qui sort d'une machine et pourquoi on a renoncé, montre un
jugement d'ingénieur. Et il reste vrai le jour où la décision s'inverse.

**A n'est pas « ne rien faire ».** A implique, dès le premier lot livré :

1. Une page de versions (GitHub Releases) avec l'installeur.
2. Une **somme SHA-256 publiée à côté** — en sachant ce qu'elle vaut : elle
   protège d'un téléchargement corrompu, **pas** d'un attaquant qui contrôle la
   page, puisqu'il modifierait aussi la somme. C'est un contrôle d'intégrité,
   pas d'authenticité. Le dire, plutôt que de laisser croire.
3. **Le numéro de version affiché dans la page Aide** (lot 2). C'est ce qui rend
   la mise à jour manuelle praticable : sans lui, l'utilisateur ne peut pas
   comparer, et « mise à jour manuelle » devient « pas de mise à jour ».
4. `docs/RELEASES.md` : la procédure de publication, exécutable par quelqu'un
   d'autre.

**Le déclencheur de B, écrit pour ne pas être oublié** — dès que l'une de ces
trois conditions est vraie :

- Cliché est installée sur une machine que Thierry n'ouvre pas chaque semaine ;
- une personne autre que Thierry l'a installée ;
- un défaut de sécurité a été corrigé et une version antérieure circule.

Ce jour-là : B, opt-in, jamais C. Et §7.2 donne d'avance la liste des fichiers.

### Niveau de risque résiduel de la recommandation

**FAIBLE aujourd'hui.** Détaillé, sans arrondir :

| Risque | Gravité | Pourquoi c'est acceptable — ou pas |
| --- | --- | --- |
| Aucun canal pour pousser un correctif de sécurité | **Moyenne, croissante** | Nulle aujourd'hui : le seul utilisateur est celui qui écrit le correctif. Devient sérieuse dès la deuxième installation. **C'est le risque qui porte toute la décision** — le déclencheur ci-dessus est là pour ça. |
| Une réinstallation manuelle le jour du passage à B | **Faible** | Borné, connu d'avance, une machine. Ce serait cher avec 500 installations ; ce n'est rien avec une. |
| Une version ancienne reste en service par oubli | **Faible** | Le numéro affiché dans l'Aide (point 3) est la parade, et elle est manuelle donc faillible. Risque assumé. |
| SmartScreen avertit à chaque installation | **Très faible** | Une fois par version, par l'auteur du binaire. Voir §5.3. |
| La somme SHA-256 donne un faux sentiment d'authenticité | **Faible** | Neutralisé en l'écrivant explicitement dans `docs/RELEASES.md`, comme au point 2. |

**Ce que la recommandation ne couvre pas** : elle ne dit rien du jour où Cliché
sera distribuée. Ce jour-là, deux décisions arrivent **ensemble** — l'updater
(B) et la signature de code (§5.3) — et la seconde est la plus urgente des deux.

---

## 7. Ce que ça changerait dans le dépôt

Listes exactes. **Aucun de ces fichiers n'a été créé ou modifié par ce document.**

### 7.1 Pour appliquer A (la recommandation)

| Fichier | Nature | Ce qu'il porterait |
| --- | --- | --- |
| `docs/UPDATES.md` | **créé** (ce fichier) | La décision et son raisonnement. |
| `docs/RELEASES.md` | à créer | Procédure de publication : commandes exactes, calcul et publication du SHA-256, ce que la somme prouve et ne prouve pas, retour arrière. |
| `README.md` | à modifier | Une section « Updates » : pas d'updater, décision assumée, renvoi vers ce document, où lire le numéro de version. La phrase *« no HTTP client in the dependency tree »* **reste vraie sous A** et ne bouge pas. |
| `docs/PLAN.md` | à modifier | Une ligne dans le tableau « Décisions arrêtées le 2 septembre 2026 » : *Mise à jour — aucune en v1, opt-in au premier utilisateur distant*. Plus une entrée « Distribution » dans le découpage en lots. |
| `src/pages/Aide.tsx` | à modifier, **lot 2** | Afficher le numéro de version. Il vient de `tauri.conf.json` via l'API applicative — donc **une seule source**, cohérent avec la règle « zéro recopie » déjà posée pour les raccourcis au lot 2. |

**Non touchés sous A** : `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`,
`package.json`, `src-tauri/capabilities/`. Le code ne bouge pas.

### 7.2 Pour appliquer B, le jour venu — le coût rendu visible

| Fichier | Nature | Ce qu'il porterait |
| --- | --- | --- |
| `src-tauri/Cargo.toml` | modifié | `tauri-plugin-updater` sous condition de cible desktop. Entraîne `reqwest` et ~15 crates transitives. |
| `src-tauri/src/lib.rs` | modifié | Enregistrement du plugin sous `#[cfg(desktop)]`. |
| `src-tauri/tauri.conf.json` | modifié | `bundle.createUpdaterArtifacts` à `true` (défaut `false`, vérifié au schéma), `plugins.updater.pubkey` (valeur littérale, jamais un chemin), `plugins.updater.endpoints`. |
| `src-tauri/capabilities/*.json` | modifié | La permission de l'updater. Le fichier est aujourd'hui vide de plugins **volontairement** ; ce serait la première entrée. |
| `package.json` | modifié | `@tauri-apps/plugin-updater` côté frontend. |
| `src/pages/…` | modifié | Le bouton « Vérifier les mises à jour » et ses **cinq états**, dont l'échec réseau silencieux. |
| **`README.md`** | modifié | **Retirer** *« no HTTP client in the dependency tree »*. Non négociable : la phrase serait fausse. |
| `.github/workflows/release.yml` | à créer | Signature via `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` depuis les secrets. Jamais dans un fichier, jamais dans un log, jamais dans une URL. |
| `docs/RELEASES.md` | modifié | La procédure devient : signer, publier le manifeste, **vérifier depuis une installation réelle** que la mise à jour est bien détectée. Un manifeste publié non testé n'est pas une release. |
| *(hors dépôt)* | — | La clé privée dans le coffre SecretStore `DevContext`, et **sauvegardée** : voir §3.3 pour ce que coûte sa perte. |
| *(hors dépôt)* | — | Le dépôt GitHub **public**, ou des artefacts publics. Voir §4.5. |

Onze emplacements pour B, contre cinq pour A dont trois sont de la documentation.
Ce n'est pas un argument décisif à lui seul — mais c'est la mesure honnête de
l'écart.

---

## 8. NON VÉRIFIÉ

Tout ce qui suit est soit une déduction, soit une source secondaire, soit une
lacune. Rien ici ne doit être cité comme un fait établi.

1. **La CSP ne bloquerait pas la requête de l'updater** (§0). Déduit de deux
   choses : `reqwest` est une dépendance **Rust** du plugin (mesuré), et la CSP
   d'un document est appliquée par le moteur du webview. Je n'ai **pas** monté
   le plugin pour l'observer. Vérifiable en une heure : brancher l'updater sur
   un endpoint local et regarder si la requête part malgré la CSP.
2. **Le format exact du manifeste, les variables `{{current_version}}` /
   `{{target}}` / `{{arch}}`, le `204 No Content`, `installMode`, et le fait que
   Windows ferme l'application pendant l'installation** viennent de la page
   `v2.tauri.app/plugin/updater/`, lue le 2 septembre 2026 **via un outil de
   récupération qui résume**. Les noms de champs sont cohérents avec le schéma
   officiel que j'ai téléchargé, mais je n'ai pas lu le HTML brut de la page :
   une citation mot pour mot pourrait être une reformulation.
3. **La perte de clé rend les installations existantes définitivement
   orphelines** (§3.3). Conclusion **structurelle**, tirée du fait que `pubkey`
   est compilée dans le binaire et que minisign n'a pas de révocation. **Je n'ai
   pas exécuté ce scénario.** Il se teste réellement : deux paires de clés, une
   installation, un manifeste signé avec la mauvaise.
4. **L'inexistence d'un mécanisme de rotation de clé** dans le plugin. Déduit de
   la forme de la configuration (`pubkey`, au singulier). **Je n'ai pas lu le
   code source** de `tauri-plugin-updater`. Si une liste de clés était acceptée,
   §3.3 et §3.4 s'adouciraient nettement — cela mérite une lecture du code avant
   d'adopter B.
5. **Le texte exact des boîtes de dialogue Windows** — « Windows a protégé votre
   ordinateur », « Éditeur inconnu » dans l'UAC, l'emplacement du bouton
   « Exécuter quand même ». La page Microsoft citée décrit un avertissement,
   elle n'en donne pas le libellé. **Se vérifie en 10 minutes** : produire
   l'installeur NSIS, le télécharger via un navigateur pour qu'il porte la
   marque du web, et le lancer.
6. **L'exigence CA/Browser Forum de juin 2023** (clé sur matériel FIPS 140-2
   niveau 2 / CC EAL 4+). Provient d'un agrégat de résultats de recherche du
   2 septembre 2026, **pas** du document du CA/B Forum.
7. **L'affirmation que Microsoft aurait retiré en mars 2024 le statut SmartScreen
   distinct des certificats EV** (§5.2). Provient d'une **page commerciale de
   SSL.com**, pas de Microsoft. C'est un tiers qui rapporte la politique d'un
   autre. Argument central du §5.3 — **à confirmer chez Microsoft avant de s'en
   servir pour justifier une dépense**.
8. **Les tarifs des certificats** : relevés le 2 septembre 2026 sur des pages de
   vente. Ce sont des prix affichés, pas des devis, et ils bougent.
9. **Le compteur de téléchargements par artefact exposé publiquement par l'API
   GitHub** (§1.3). **Non mesuré.** Je n'ai pas observé de champ
   `download_count` — la release que j'ai interrogée n'exposait aucun artefact.
10. **La politique de conservation des journaux d'accès de GitHub** — combien de
    temps l'IP d'un visiteur est gardée, et à qui elle est transmise. §1.4 cite
    leur documentation sur le **fait** de journaliser ; je n'ai rien vérifié sur
    la **durée**.
11. **L'`User-Agent` réellement émis** par `reqwest` dans ce plugin, et
    l'éventualité qu'il soit personnalisé par Tauri. §1.1 dit « celui de reqwest
    par défaut » : c'est une hypothèse. Se lit dans le code du plugin.
12. **L'impact réel sur le temps de démarrage** d'une vérification au lancement
    (§ option C). Aucun chiffre mesuré, et il ne le sera pas : l'option est
    écartée. La cible de 150 ms du lot 1 rend l'argument plausible, pas prouvé.
13. **Le comportement de `bundle.createUpdaterArtifacts` sur cette machine.**
    La clé et sa description viennent du schéma officiel ; je n'ai lancé aucun
    build avec, d'autant que `src-tauri/icons/` est vide et que le build échoue
    tant que les icônes ne sont pas générées (`README.md`).

---

## Sources

Toutes consultées le **2 septembre 2026**.

| Source | Nature | Ce qu'elle établit ici |
| --- | --- | --- |
| API crates.io, `/crates/tauri-plugin-updater` | primaire, mesurée | 2.11.0, publiée le **2026-08-31T11:21:56Z**, **9 004 138** téléchargements cumulés sur le crate dont **14 299** sur 2.11.0 |
| API crates.io, `.../2.11.0/dependencies` | primaire, mesurée | `reqwest ^0.13`, `minisign-verify ^0.2`, `tauri ^2.10` |
| `pnpm tauri signer --help` (CLI 2.11.4, **cette machine**) | primaire, exécutée | sous-commandes `sign` / `generate` et leurs options |
| `https://schema.tauri.app/config/2` | primaire, téléchargé | `bundle.createUpdaterArtifacts` (défaut `false`) ; `bundle.windows.{certificateThumbprint, digestAlgorithm, timestampUrl, signCommand, tsp}` |
| `https://v2.tauri.app/plugin/updater/` | primaire, **lue via résumé** | manifeste, endpoints dynamiques, `pubkey`, variables d'environnement de signature |
| `api.github.com` en anonyme, deux dépôts | primaire, mesurée | dépôt public → **200** ; dépôt privé → **404** |
| Microsoft Learn, *Defender SmartScreen overview* (`ms.date` 23 avril 2026) | primaire | réputation par volume de téléchargements ; avertissement si le fichier est inconnu |
| GitHub Docs, *What is GitHub Pages* / *Changing the visibility…* | primaire | journalisation de l'IP des visiteurs ; publication privée réservée à Enterprise Cloud |
| SSL.com, page produit EV code signing | **commerciale**, primaire pour son tarif | 349 $/an ; YubiKey +379 $ ; l'affirmation sur mars 2024 (§8, point 7) |
| Recherche web, comparateurs de certificats | **secondaire, agrégée** | OV dès ~219 $/an ; exigence matérielle CA/B Forum (§8, point 6) |
| État du dépôt (`git log`, `git remote`, lecture des fichiers) | primaire, mesurée | aucun commit, aucun remote ; CSP, cibles de bundle, `.gitignore` lignes 25-26 |
