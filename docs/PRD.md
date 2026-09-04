# Cliché — ce que ça doit faire, et pour qui

> Document produit, en français (langue d'échange du projet, comme `docs/PLAN.md`).
> Le code, les commentaires, les commits et le `README` restent en anglais.
>
> Établi le **2 septembre 2026**. Toute date ci-dessous est absolue.
>
> **Ce document ne dit pas comment on construit.** Le découpage en lots, l'ordre des
> travaux, les fichiers et les parades techniques vivent dans `docs/PLAN.md`. Ici :
> pour qui, pourquoi, et à quoi on reconnaîtra que c'est réussi.
>
> Rédigé par l'agent Architecte, contrôlé et posé par Jarvis le 2 septembre 2026.

---

## 1. À qui ça sert

Un **développeur solo**, sur sa propre machine Windows, qui produit chaque jour des
images de son écran pour trois raisons : **prouver** (un bug, un état, une erreur),
**documenter** (une note, un README, un billet), **transmettre** (coller dans une issue,
un salon Discord, un message).

Ce n'est pas un profil marketing, c'est une contrainte : il n'y a **ni équipe, ni revue,
ni support**. Chaque friction est payée par la seule personne qui utilise l'outil, et
chaque fuite de donnée l'est aussi.

### Les situations où l'outil est dégainé

- **Quelque chose casse et ne restera pas cassé longtemps.** Une exception dans un
  terminal, un rendu tordu, une notification qui disparaît en trois secondes. La capture
  doit partir *avant* que l'écran change — c'est la seule situation du lot où la latence
  est un critère fonctionnel et pas un confort.
- **Une note est en train de s'écrire.** L'image sert d'illustration ; elle sera relue
  dans six mois. Elle doit être retrouvable, pas seulement collée.
- **Quelque chose va être publié ou envoyé à un tiers.** L'image contient un chemin, un
  jeton, une adresse e-mail, un nom de client. Il faut masquer **avant** d'envoyer, et
  le masquage doit résister à quelqu'un qui ouvre le fichier avec un autre outil.
- **Une documentation longue doit être conservée hors ligne**, en une seule image, sans
  reconstituer douze captures à la main.
- **Une preuve d'il y a trois jours redevient utile.** « J'avais capturé ça, où est-ce
  passé ? » — la réponse ne doit pas être « dans un dossier de 400 fichiers nommés
  `Capture d'écran 2026-08-29 143012.png` ».

### Ce qui est vrai de cette personne, et qui commande tout le reste

1. **Ses captures sont parmi les fichiers les plus sensibles de sa machine.** Un écran
   contient ce qui était affiché : gestionnaire de mots de passe ouvert, jeton dans un
   terminal, adresse d'un client, conversation privée. Un dossier de captures est un
   dossier de secrets accidentels.
2. **Elle ne veut aucun service en ligne.** Pas de compte, pas de lien, pas de « votre
   image est prête à être partagée ». La contrainte de confidentialité est donc
   **entièrement locale** : rétention, effacement, et le droit de ne rien enregistrer.
3. **Elle n'a personne à qui demander.** L'aide intégrée est le seul support existant.
   C'est un livrable de premier rang, pas une finition.

---

## 2. Cas d'usage

Chaque cas : **déclencheur → gestes → résultat attendu**. Ce sont des parcours, pas des
fonctionnalités.

### Cas 1 — Une erreur apparaît, et elle ne va pas rester

**Déclencheur** : une stack trace s'affiche dans un terminal, ou un composant se rend
mal. L'état est volatil : un rafraîchissement, et il a disparu.

**Gestes** : raccourci global → un voile assombrit l'écran gelé → glisser un rectangle
autour de la zone → relâcher → **`Entrée`** (ou double-clic dans la sélection).

> **Modifié le 4 septembre 2026, et c'est un recul assumé sur la vitesse.** Jusque-là,
> relâcher copiait. Relâcher **pose** désormais la sélection : on peut la
> redimensionner par ses huit poignées et la déplacer avant de valider, parce qu'un
> rectangle mal tracé coûtait un retracé complet. Le prix est **un geste de plus sur
> CHAQUE capture**, y compris celles qui étaient déjà bonnes. Décision de Thierry ;
> l'alternative écartée était de garder « relâcher copie » et de n'ouvrir l'édition
> qu'en tenant `Maj` au relâchement — chemin rapide intact, mais il faut le savoir, et
> au moment où l'on relâche on n'a pas toujours vu que le rectangle est mauvais.
> Réversible : la bascule est dans `src/veil/main.ts`.

**Résultat attendu** : l'image est dans le presse-papier **à la validation** — une
frappe après le relâchement. `Ctrl+V` dans une issue GitHub la colle. Aucune fenêtre à chercher, aucun
bouton « copier » à trouver. Si l'utilisateur ne fait rien d'autre, il ne reste rien à
ranger.

### Cas 2 — Il faut cette fenêtre-là, et rien d'autre

**Déclencheur** : besoin de montrer une boîte de dialogue ou une fenêtre d'application
précise, proprement détourée, alors qu'elle est **partiellement recouverte** par une
autre.

**Gestes** : raccourci → passer la souris sur la fenêtre voulue → son contour se
surligne → clic.

**Résultat attendu** : l'image contient exactement cette fenêtre, entière, **sans la
portion qui la recouvrait** et sans bordure parasite ajoutée par le système.

### Cas 3 — Il y a un secret sur l'image, et elle va sortir de la machine

**Déclencheur** : la capture est faite, elle est bonne, mais on y lit une clé d'API, une
adresse e-mail ou le nom d'un client. Elle est destinée à un salon Discord ou à un
billet public.

**Gestes** : la capture s'ouvre dans l'éditeur → outil flou → tracer sur la zone
sensible → exporter en PNG ou copier.

**Résultat attendu** : sur le fichier obtenu, **le texte masqué est illisible et
irrécupérable**. Ouvrir ce PNG dans un autre éditeur, jouer sur les niveaux, extraire
les couches : rien ne ramène le texte. Le flou est appliqué aux pixels, il n'est pas
dessiné par-dessus une image intacte.

### Cas 4 — Capturé aujourd'hui, retrouvé jeudi

**Déclencheur, temps 1** : pendant une session de travail, trois ou quatre captures sont
prises comme aide-mémoire. Aucune n'est annotée, aucune n'est envoyée.

**Gestes, temps 1** : raccourci, rectangle, relâcher, `Entrée`. Rien de plus — voir
le cas 1 pour ce que ce quatrième geste a acheté, et ce qu'il coûte. Les images sont
rangées automatiquement (ou pas du tout, si l'utilisateur a coupé l'enregistrement
automatique).

**Déclencheur, temps 2, trois jours plus tard** : une note est en cours de rédaction, il
faut « celle du mardi avec le graphe ».

**Gestes, temps 2** : ouvrir la bibliothèque → filtrer par date → reconnaître la
vignette → copier l'image ou retrouver son fichier.

**Résultat attendu** : l'image est retrouvée sans ouvrir l'explorateur de fichiers et
sans se souvenir d'un nom. Elle a survécu à un redémarrage de l'application et de la
machine.

### Cas 5 — Une page de documentation plus haute que l'écran

**Déclencheur** : une page web longue doit être conservée en entier, hors ligne, en une
seule image.

**Gestes** : viser la page dans le navigateur → déclencher la capture défilante →
l'outil fait défiler et recolle.

**Résultat attendu** : une image unique, sans bande dupliquée ni en-tête collant répété
à chaque jointure. **Si le résultat n'est pas fiable, l'outil le dit avant de rendre
l'image** — il vaut mieux « recollage incertain, vérifiez » qu'une image fausse qu'on
croit bonne.

### Cas 6 — ÉCHEC : la zone tracée est mauvaise

**Déclencheur** : la souris a glissé, le rectangle est décalé, ou il manque une ligne en
bas. Variante fréquente : le bouton a été relâché par accident, presque tout de suite.

**Gestes attendus, depuis le 4 septembre 2026** : le rectangle relâché est **posé, pas
validé** — on le corrige sur place, en tirant l'une de ses huit poignées ou en le
saisissant à l'intérieur pour le déplacer, puis `Entrée`. C'est la réponse de premier
rang à ce cas, et c'est ce que les poignées ont acheté.
`Échap` reste la sortie : il referme le voile **sans rien capturer et sans rien
enregistrer**, et le raccourci relance immédiatement une sélection propre. Un rectangle
d'aire nulle ou dérisoire (un simple clic) ne produit **pas** de fichier ni d'entrée
dans la bibliothèque.

**Résultat attendu** : se tromper coûte une seconde et ne laisse **aucune trace** — ni
fichier orphelin, ni presse-papier écrasé par une image vide, ni ligne parasite dans
l'historique. C'est ce qui rend l'outil réutilisable sans hésiter.

### Cas 7 — ÉCHEC : le raccourci ne fait rien

**Déclencheur** : on appuie, rien ne se passe. La combinaison est déjà prise — par
l'Outil Capture de Windows, par Discord, par OBS, par un pilote de clavier. C'est un
échec **silencieux** par nature : le système attribue la touche à un autre programme et
n'en informe personne.

**Gestes attendus** : ouvrir l'application → un état visible signale que **ce raccourci
n'a pas pu être enregistré**, en le nommant → l'aide intégrée explique la cause probable
et propose une combinaison de repli → changer la combinaison depuis l'application.

**Résultat attendu** : l'utilisateur comprend en moins d'une minute que **ce n'est pas
Cliché qui est cassé**, et s'en sort seul. Un raccourci refusé est un **message**, jamais
un silence.

### Cas 8 — ÉCHEC : l'application est déjà ouverte

**Déclencheur** : le lanceur est cliqué une deuxième fois, ou l'application est déjà
active en arrière-plan depuis le démarrage de la machine.

**Gestes attendus** : la fenêtre existante revient au premier plan. Aucune deuxième
instance ne démarre.

**Résultat attendu** : **un seul processus, donc un seul propriétaire des raccourcis
globaux et une seule base de la bibliothèque**. Deux instances signifieraient une
seconde qui ne reçoit aucun raccourci (le système les a donnés à la première) et deux
écritures concurrentes sur les mêmes données. C'est un cas d'échec produit, pas un
détail technique.

---

## 3. Périmètre

### INDISPENSABLE v1 — sans ça, l'outil ne remplace pas ce qui existe déjà

| Capacité | Pourquoi elle est indispensable |
| --- | --- |
| Capture de zone au raccourci global | C'est le geste, tout le reste est autour |
| Voile de sélection sur écran gelé, `Échap` pour annuler | Se tromper doit être gratuit (cas 6) |
| Capture d'écran entier | Le cas « je montre tout » sans tracer |
| Capture d'une fenêtre désignée | Détourage propre impossible à la main (cas 2) |
| Copie automatique dans le presse-papier | Le trajet capture → collage doit être sans étape |
| Annotation : flèche, rectangle, texte, surlignage | Une preuve sans pointeur ne prouve rien |
| **Flou destructif** | Sans lui, l'outil est dangereux dès qu'on partage (cas 3) |
| Annuler / rétablir | Annoter sans filet fait annoter moins |
| Enregistrement PNG + interrupteur « ne pas enregistrer automatiquement » | Le droit de ne pas laisser de trace est une exigence, pas une préférence |
| Bibliothèque locale avec vignettes et recherche par date | Sinon on retombe sur un dossier illisible (cas 4) |
| Rétention et effacement définitif | Un historique de captures est un stock de secrets |
| Registre unique des raccourcis + **aide intégrée qui en dérive** | Seul support existant ; une liste recopiée ment au bout d'une semaine |
| Signalement d'un raccourci refusé | Échec silencieux inacceptable (cas 7) |
| Instance unique | Deux instances = raccourcis et base en conflit (cas 8) |
| Thème clair et sombre, utilisable jusqu'à 480×600 | L'outil vit à côté d'autres fenêtres, jamais en plein écran |
| Installeur Windows 11 x64 | Sans installeur il n'y a pas de produit |
| Zéro réseau, démontrable | C'est la promesse centrale, elle doit être vérifiable |

### SOUHAITABLE v1 — améliore nettement, ne bloque pas la livraison

| Capacité | Pourquoi souhaitable et pas indispensable |
| --- | --- |
| Capture d'une page web défilante | Fort besoin, mais technique fragile ; livrable marqué « expérimental » si le résultat n'est pas fiable |
| Dimensions et coordonnées affichées pendant le tracé | Rend le pixel exact contrôlable à l'œil |
| Loupe au curseur pour viser au pixel | Utile sur les bordures, pas vital |
| Capture différée de quelques secondes | Seule façon de capturer un menu ouvert |
| Copier le chemin du fichier enregistré | Raccourci vers l'insertion dans une note |
| Glisser-déposer d'une capture vers une autre application | Confort réel, dépend du comportement de l'application cible |
| Export JPEG / WebP | PNG suffit pour tout ce qui compte ici |
| Purge automatique après N jours | La rétention manuelle couvre le besoin minimal |

### PLUS TARD — reconnu utile, délibérément repoussé

| Capacité | Raison du report |
| --- | --- |
| Preuve du comportement multi-écran et DPI mixtes | Impossible à vérifier sur cette machine (un seul écran, voir §7) : c'est une dette matérielle, pas un manque de code |
| Pipette de couleur / mesure de distance | Un autre métier que la capture |
| Numérotation d'étapes, modèles d'annotation | Utile pour de la documentation en série, pas pour l'usage quotidien |
| Renommage automatique par modèle | Suppose que la bibliothèque ne suffit pas ; à réévaluer après usage réel |
| Icône en zone de notification / démarrage avec Windows | Change le modèle de cycle de vie de l'application ; à trancher (voir §7) |
| Capture d'une **fenêtre** défilante quelconque | Voir §4 |

---

## 4. Hors périmètre — ce qu'on ne fera pas, et pourquoi

Cette section n'est pas une liste d'idées en attente. C'est une liste de **refus**.

- **Enregistrement vidéo ou GIF.** Ce n'est pas la même application : encodage, son,
  fréquence d'images, gestion de fichiers lourds, et une interface entièrement
  différente. L'ajouter diluerait le seul critère qui compte ici — le trajet raccourci →
  presse-papier en moins d'une seconde. Windows 11 embarque déjà un enregistreur.
- **OCR (extraire le texte d'une capture).** Suppose soit un service en ligne — exclu par
  la promesse — soit un modèle embarqué qui pèserait plus que toute l'application, pour
  un besoin qui n'apparaît dans aucun des huit cas d'usage.
- **Partage en ligne, lien public, envoi vers un service tiers.** C'est la promesse
  centrale, inversée. Un outil qui sait publier une capture est un outil qui peut publier
  un mot de passe par erreur, et il n'y a pas de retour en arrière sur une image publiée.
- **Compte utilisateur.** Rien à protéger côté serveur puisqu'il n'y a pas de serveur.
  Un compte n'apporterait que de la surface d'attaque et des données personnelles à
  stocker.
- **Synchronisation cloud.** Synchroniser un dossier de captures d'écran, c'est copier
  ses secrets accidentels sur une machine dont on ne contrôle rien. Le stockage reste
  local ; l'utilisateur reste libre de placer son dossier où il veut, en connaissance de
  cause.
- **Télémétrie, rapport d'erreur distant, vérification de mise à jour.** Même raison,
  et ils feraient mentir la phrase « ne parle à aucun réseau ». Les diagnostics restent
  sur la machine. La décision complète sur la mise à jour est arbitrée dans
  `docs/UPDATES.md`.
- **macOS et Linux — pas livrés en v1, mais la porte reste ouverte, et c'est une
  contrainte, pas un espoir.** *Corrigé le 3 septembre 2026 : cette entrée affirmait
  que « le code de capture est irréductiblement lié aux API Windows ». C'était faux.
  `xcap` est multiplateforme (dépendances `objc2` sous macOS, `libwayshot`/`pipewire`
  sous Linux, toutes conditionnées par cible) et notre propre source ne contient aucun
  `cfg(windows)` ni aucun appel Win32 direct.* On ne livre que **Windows 11 x64**,
  parce qu'aucune machine ne permet d'éprouver les autres — annoncer un système non
  testé serait une promesse fausse. Mais **rien ne doit fermer la porte** :
  - une bibliothèque Windows seule ne peut jamais être le **seul** chemin d'une
    capacité ;
  - si un morceau devient natif (voile, saisie, presse-papier), c'est **une interface
    unique et un moteur par système**, jamais un embranchement dispersé dans le code ;
  - tout morceau natif arrive avec **son plan de portage écrit à côté**, sans quoi il
    n'est pas fini.
  L'état réel de la portabilité, vérifié plutôt que supposé, est tenu dans
  `docs/STACK.md`.
- **Capture d'une fenêtre défilante quelconque.** Le défilement v1 vise **les pages
  web**. Le recollage générique repose sur la simulation de molette et la corrélation
  d'images : il rate sur les listes virtualisées, les en-têtes collants et le défilement
  animé, et il rate **en produisant une image plausible mais fausse**. Un résultat faux
  qui a l'air juste est pire que l'absence de fonctionnalité.

---

## 5. Exigences non fonctionnelles

Chaque exigence porte un **seuil** ou un **comportement observable**. Convention :
`[MESURÉ]` = valeur constatée sur cette machine ; `[EXIGÉ]` = seuil que le produit doit
atteindre, **pas encore mesuré** au 2 septembre 2026.

### Performance

- **P1** `[EXIGÉ]` Du raccourci à l'apparition du voile : **médiane < 150 ms sur
  10 mesures**, sur 1920×1080 à 100 %. Journalisé, pas estimé à l'œil.
- **P2** `[EXIGÉ]` Le relâchement de la souris et la disponibilité de l'image dans le
  presse-papier sont perçus comme un seul geste : **< 500 ms** pour une zone allant
  jusqu'au plein écran. *Seuil proposé, à confirmer par Thierry — voir §7.*
- **P3** `[EXIGÉ, observable]` Aucune fenêtre de voile n'est **créée** au moment du
  raccourci : elle préexiste. Vérifiable dans les journaux de démarrage (création du
  voile) puis à chaque déclenchement (affichage seulement).
- **P4** `[EXIGÉ, observable]` L'image ne transite jamais entre le cœur natif et
  l'interface sous forme de texte encodé. Vérifiable par lecture du contrat d'échange :
  une capture plein écran ne doit pas produire de charge utile textuelle de plusieurs
  méga-octets.
- **P5** `[EXIGÉ, observable]` L'application au repos, sans fenêtre visible, ne consomme
  pas de temps processeur mesurable au-delà du bruit de fond : **0–1 % dans le
  Gestionnaire des tâches sur une minute d'observation**. Elle attend un raccourci, elle
  ne sonde rien.

### Confidentialité — la contrainte structurante

- **C-a** `[EXIGÉ, observable]` **Zéro connexion sortante.** Pendant 10 minutes d'usage
  réel (captures, annotations, enregistrements), une observation du trafic du processus
  ne montre **aucune connexion sortante**. Aucune exception, pas même une résolution DNS.
- **C-b** `[MESURÉ le 2 septembre 2026, par lecture de `src-tauri/tauri.conf.json`]` La
  politique de sécurité du contenu de l'interface limite `connect-src` au canal interne
  (`ipc: http://ipc.localhost`), et pose `object-src 'none'` et `form-action 'none'`.
  Toute modification élargissant `connect-src` est un changement de contrat produit, pas
  un réglage.
- **C-c** `[EXIGÉ, observable]` Un interrupteur **« ne pas enregistrer
  automatiquement »** existe, est atteignable en moins de deux clics depuis la fenêtre
  principale, et **est respecté** : activé, une capture copiée dans le presse-papier ne
  crée **aucun fichier ni aucune entrée** dans la bibliothèque.
- **C-d** `[EXIGÉ, observable]` **L'effacement efface.** Supprimer une capture depuis la
  bibliothèque retire : l'entrée, la vignette, **et le fichier image sur le disque**.
  Vérifiable en cherchant le fichier après coup. Aucune corbeille interne cachée.
- **C-e** `[EXIGÉ, observable]` Une **rétention** est réglable et l'emplacement du
  dossier de captures est **affiché en clair** dans l'application. L'utilisateur ne doit
  jamais avoir à deviner où ses secrets accidentels sont posés.
- **C-f** `[EXIGÉ, observable]` Les journaux de diagnostic ne contiennent **ni chemin de
  fichier utilisateur complet, ni titre de fenêtre capturée, ni contenu d'image**. Un
  titre de fenêtre suffit à révéler un nom de client ou un sujet de conversation.
- **C-g** `[EXIGÉ, observable]` Le masquage est **destructif dans le fichier livré** :
  aucune couche, aucun calque, aucune miniature intégrée ne conserve la zone d'origine.
  Vérifiable en inspectant le PNG exporté avec un outil tiers.

### Robustesse

- **R1** `[EXIGÉ, observable]` `Échap` referme le voile sans capturer, sans écrire, sans
  toucher au presse-papier — depuis n'importe quel état de la sélection.
- **R2** `[EXIGÉ, observable]` Un échec de capture (API refusée, fenêtre disparue en
  cours de route) affiche un **message compréhensible nommant ce qui a échoué**.
  L'application reste utilisable : pas de fermeture, pas de fenêtre morte.
- **R3** `[EXIGÉ, observable]` **Instance unique** : lancer l'application une seconde
  fois ramène la fenêtre existante au premier plan. Vérifiable : un seul processus dans
  le Gestionnaire des tâches.
- **R4** `[EXIGÉ, observable]` Un raccourci **refusé par le système** est signalé dans
  l'interface, en nommant la combinaison. Jamais d'échec silencieux.
- **R5** `[EXIGÉ, observable]` La bibliothèque survit à une **fermeture brutale** :
  après arrêt forcé du processus puis redémarrage, les captures enregistrées avant
  l'arrêt sont toutes présentes, et aucune entrée ne pointe vers un fichier absent.
- **R6** `[EXIGÉ, observable]` Annuler / rétablir tient sur **20 opérations d'annotation
  consécutives** sans perte ni désordre.
- **R7** `[EXIGÉ, observable]` En version livrée (console masquée), les diagnostics
  restent consultables **depuis la machine** : un fichier journal local existe et
  l'application indique où il se trouve. Sinon, un incident chez l'utilisateur est
  indiagnostiquable — et il n'y a pas de rapport distant pour rattraper.

### Accessibilité

- **A1** `[EXIGÉ, observable]` **Tout est atteignable au clavier** : chaque action de la
  fenêtre principale, de l'éditeur et de la bibliothèque est accessible par `Tab` /
  flèches / `Entrée`, sans souris.
- **A2** `[EXIGÉ, observable]` L'élément focalisé est **visible en permanence**, dans les
  deux thèmes. Aucun `outline: none` sans remplacement.
- **A3** `[EXIGÉ, mesurable]` Contrastes conformes **WCAG 2.2 niveau AA** : **4,5:1**
  pour le texte courant, **3:1** pour le texte large et les éléments d'interface
  porteurs de sens. Mesuré sur l'écran qui tourne, dans les deux thèmes — pas déduit des
  jetons de couleur. Le verre translucide est précisément ce qui fait échouer ce point :
  un fond variable derrière un texte fixe.
- **A4** `[EXIGÉ, observable]` **Aucune information portée par la seule couleur** : un
  état actif, une erreur ou une sélection porte aussi une forme, une icône ou un texte.
- **A5** `[EXIGÉ, observable]` `prefers-reduced-motion` est respecté : animations
  d'apparition supprimées quand le système le demande.
- **A6** `[EXIGÉ, observable]` Le voile de sélection reste utilisable **au clavier** :
  déplacement et validation d'une sélection sans souris, ou, si c'est jugé hors
  périmètre v1, l'aide le dit explicitement. *À trancher — voir §7.*

### Internationalisation

- **I1** `[EXIGÉ, observable]` **Aucune chaîne visible n'est écrite en dur** dans un
  composant d'interface : toutes proviennent d'un catalogue unique. Contrôle : une
  recherche de texte littéral dans les composants ne rend rien d'affiché à l'écran.
- **I2** `[EXIGÉ]` La langue de l'interface v1 est **unique et assumée** — français ou
  anglais, à trancher (§7) — mais la structure permet d'en ajouter une sans toucher aux
  composants.
- **I3** `[EXIGÉ, observable]` Les dates affichées suivent les **conventions locales de
  la machine** (Belgique : jour/mois/année, semaine commençant le lundi). Une
  bibliothèque filtrée par date est inutilisable si les dates sont au format d'un autre
  pays.
- **I4** `[EXIGÉ, observable]` Les noms de fichiers générés ne contiennent **aucun
  caractère interdit par Windows** (`\ / : * ? " < > |`) ni accent susceptible de casser
  un partage ultérieur. Vérifiable : capture dont le titre de fenêtre source contient
  `:` et `?` → le fichier est créé sans erreur.
- **I5** `[EXIGÉ, observable]` L'interface ne casse pas si une traduction est **30 %
  plus longue** : pas de texte tronqué sans indication, pas de bouton débordant.

### Compatibilité Windows

- **W1** `[MESURÉ le 2 septembre 2026]` Cible : **Windows 11 x64**. Machine de
  développement : Windows 11 Pro 10.0.26200, WebView2 152.0.4191.53 présent.
- **W2** `[EXIGÉ, observable]` Le processus est déclaré **per-monitor DPI aware v2**.
  Observable au démarrage : le journal annonce le nombre d'écrans, la résolution
  physique et le facteur d'échelle. Constaté sur cette machine le 2 septembre 2026 :
  `1 display, 1920x1080 physical px at (0, 0), scale 1.00`.
- **W3** `[EXIGÉ, observable]` L'installation et le premier lancement se font sur un
  **compte Windows standard**, sans élévation de privilèges. Une capture d'écran ne
  justifie aucun droit administrateur.
- **W4** `[EXIGÉ, observable]` L'application démarre sur une machine **sans chaîne
  d'outils de développement** (ni Rust, ni Node, ni pnpm). Vérifiable en installant le
  paquet livré sur un compte Windows propre.
- **W5** `[EXIGÉ, observable]` Le comportement face à **SmartScreen** pour un exécutable
  non signé est connu et **documenté dans l'aide** : si un avertissement apparaît à la
  première installation, l'utilisateur doit savoir pourquoi avant de le voir. Analyse
  dans `docs/UPDATES.md` §5. *Le comportement exact reste à constater — §7.*
- **W6** `[EXIGÉ, observable]` Aucune dépendance à une fonctionnalité optionnelle de
  Windows non installée par défaut sur Windows 11.

---

## 6. Comment on saura que c'est réussi

Chaque critère dit **ce qu'on lance ou ce qu'on regarde**, et **ce qu'on doit voir**.
Un critère qu'on ne sait pas trancher n'est pas un critère.

### Critères validés par Thierry le 2 septembre 2026 — repris tels quels

- **C1 — latence.** *On lance* : le raccourci, 10 fois, avec journalisation du délai
  entre l'appui et l'affichage du voile, sur 1920×1080 à 100 %.
  *On doit voir* : une **médiane inférieure à 150 ms**.

- **C2 — pixel exact.** *On lance* : la capture d'une mire de dimensions connues, avec
  un rectangle de taille connue.
  *On doit voir* : un PNG dont les dimensions sont **exactement celles du rectangle
  tracé, 0 pixel d'écart**.
  *Réserve inscrite au dossier* : en **multi-écran ou DPI mixtes**, le code est écrit
  correct mais **NON VÉRIFIABLE sur cette machine**. Ce n'est pas « non testé », c'est
  « intestable ici ».

- **C3 — presse-papier.** *On lance* : une capture, puis `Ctrl+V` dans **Paint** et dans
  **Discord**.
  *On doit voir* : l'image collée dans les deux, aux bonnes dimensions.

- **C4 — flou destructif.** *On regarde* : le PNG exporté d'une zone floutée, ouvert
  dans un éditeur d'images tiers.
  *On doit voir* : un **texte illisible**, et **aucune couche réversible** subsistant
  dans le fichier. Le texte d'origine n'est récupérable par aucune manipulation de
  l'image.

- **C5 — aide dérivée.** *On lance* : l'ajout d'une entrée au registre des raccourcis,
  puis un redémarrage de l'application, **sans toucher au fichier d'aide**.
  *On doit voir* : le nouveau raccourci **présent dans l'Aide**.

- **C6 — fenêtre étroite.** *On regarde* : l'application redimensionnée à **480×600**,
  en **thème clair** puis en **thème sombre**.
  *On doit voir* : **tous les menus atteignables**, constaté par capture d'écran, dans
  les deux thèmes.

### Critères ajoutés le 2 septembre 2026 — ce qui manquait pour pouvoir dire « c'est bon »

- **C7 — zéro réseau, démontré.** *On lance* : 10 minutes d'usage réel (capture,
  annotation, enregistrement, ouverture de la bibliothèque) sous observation du trafic
  du processus (Moniteur de ressources ou équivalent).
  *On doit voir* : **aucune connexion sortante**, aucune résolution DNS.
  Sans ce critère, la promesse centrale du produit n'est jamais vérifiée, seulement
  affirmée.

- **C8 — l'effacement efface.** *On lance* : enregistrer une capture, noter le chemin du
  fichier, la supprimer depuis la bibliothèque, puis chercher ce chemin sur le disque.
  *On doit voir* : **fichier absent**, **vignette absente**, **entrée absente** de la
  bibliothèque après redémarrage de l'application.

- **C9 — « ne pas enregistrer automatiquement » est respecté.** *On lance* : activer
  l'interrupteur, faire trois captures, puis inspecter le dossier de captures et la
  bibliothèque.
  *On doit voir* : **zéro fichier créé, zéro entrée créée**, et les trois images bien
  passées par le presse-papier.

- **C10 — instance unique.** *On lance* : l'application déjà ouverte, on la relance
  depuis le menu Démarrer.
  *On doit voir* : la fenêtre existante revient au premier plan, et le Gestionnaire des
  tâches montre **un seul processus**.

- **C11 — raccourci refusé, signalé.** *On lance* : on occupe volontairement la
  combinaison avec une autre application, puis on démarre Cliché.
  *On doit voir* : un message nommant **la combinaison refusée**, et l'aide qui explique
  quoi faire. Jamais un raccourci qui ne répond pas sans un mot.

- **C12 — annulation sans trace.** *On lance* : raccourci, tracer un rectangle,
  `Échap` ; puis raccourci, simple clic sans glisser.
  *On doit voir* : **aucun fichier**, **aucune entrée en bibliothèque**, et un
  presse-papier **inchangé** (ce qu'il contenait avant est toujours collable).

- **C13 — installation sur machine nue.** *On lance* : l'installeur livré, sur un compte
  Windows 11 standard **sans Rust, Node ni pnpm**, sans droits administrateur.
  *On doit voir* : l'application démarre, capture, et copie dans le presse-papier.
  *Réserve* : **NON VÉRIFIABLE dans l'immédiat** faute d'une seconde machine ou d'une
  machine virtuelle — à inscrire comme dette, voir §7.

- **C14 — clavier et contraste.** *On regarde* : l'application parcourue uniquement au
  clavier, dans les deux thèmes, contrastes mesurés sur l'écran qui tourne.
  *On doit voir* : chaque action atteignable, focus visible partout, **4,5:1** sur le
  texte courant et **3:1** sur les éléments d'interface.

- **C15 — l'aide couvre les échecs.** *On regarde* : l'aide intégrée, en cherchant les
  trois échecs des cas 6, 7 et 8.
  *On doit voir* : **une réponse pour chacun**. Une aide qui ne documente que le chemin
  heureux ne sert à personne : personne ne consulte l'aide quand tout marche.

---

## 7. NON VÉRIFIÉ / À TRANCHER

### Ce que je n'ai pas pu établir au 2 septembre 2026

1. **Le comportement multi-écran et DPI mixtes.** La machine ne porte qu'un écran
   (`\\.\DISPLAY1`, 1920×1080 à (0,0), échelle 100 %, tour fixe MS-7D98). Tout critère
   exigeant un second écran est **NON VÉRIFIABLE ici** — C2 en particulier. Un second
   écran, même une télévision en HDMI, lèverait cette dette.
2. **Toutes les valeurs de performance.** Aucune latence, aucune consommation mémoire,
   aucune taille d'installeur n'a été mesurée à ce jour. Les seuils P2 et P5 sont des
   **exigences proposées**, pas des constats.
3. **L'existence d'une seconde machine ou d'une machine virtuelle Windows propre** pour
   le critère C13. Non vérifiée.
4. **Le comportement SmartScreen** d'un installeur non signé sur cette machine (W5).
   Non constaté.
5. **Le conflit de la touche `PrintScreen`** avec l'Outil Capture de Windows sur cette
   machine. Annoncé comme probable par le plan, **non confirmé**.
6. **Le contenu réel de l'application au-delà du squelette.** Au moment d'écrire ce
   document, la partie native se compose de l'entrée du binaire, de l'assemblage de
   l'application et de l'énumération des écrans ; l'interface, de quatre fichiers. **Il
   n'existe encore ni capture, ni éditeur, ni bibliothèque** : tout ce document décrit un
   produit à construire, et aucun de ses critères n'est aujourd'hui vert.

### Questions ouvertes — arbitrage de Thierry

1. **Langue de l'interface v1** : français ou anglais ? Le `README` est en anglais par
   convention, mais l'utilisateur unique est francophone. Cette décision commande le
   catalogue de chaînes et l'aide intégrée. *(Exigence I2.)*
2. **Le seuil P2 (relâchement → presse-papier < 500 ms)** est proposé par l'Architecte et
   n'a été validé par personne. À confirmer, corriger, ou supprimer — un seuil arbitraire
   qu'on n'assume pas vaut moins que pas de seuil.
3. **Le voile de sélection doit-il être utilisable au clavier (A6) en v1 ?** Si non,
   c'est un manque d'accessibilité assumé et l'aide doit le dire.
4. **Rétention par défaut** : les captures se gardent-elles indéfiniment, ou une purge
   automatique est-elle active dès l'installation ? Un défaut « on garde tout » est un
   stock de secrets qui grossit tout seul ; un défaut « on purge » peut détruire une
   preuve. Il faut choisir, et l'écrire dans l'aide.
5. **« Ne pas enregistrer automatiquement » : est-ce l'état par défaut ?** Question de
   confidentialité, pas de confort. Le défaut le plus prudent n'est pas le plus pratique.
6. **Emplacement par défaut du dossier de captures** — et faut-il **refuser** un dossier
   manifestement synchronisé (OneDrive, Dropbox, iCloud) ou seulement **avertir** ?
   L'utilisateur en placerait un sans y penser, et la promesse « rien ne quitte la
   machine » serait contournée par son propre système de fichiers.
7. **L'historique du presse-papier de Windows** (`Win+V`) peut conserver et, s'il est
   configuré ainsi, synchroniser ce qui y est copié. Cliché copie des images
   potentiellement sensibles. Faut-il chercher à marquer le contenu comme exclu de cet
   historique, l'avertir dans l'aide, ou considérer que c'est le domaine de
   l'utilisateur ? **Ce que Windows 11 fait par défaut sur cette machine n'a pas été
   vérifié**, ni si un tel marquage est réellement respecté. C'est un trou de
   confidentialité en dehors de notre code, et il mérite une décision explicite.
8. **Cycle de vie de l'application** : doit-elle vivre en arrière-plan (icône de
   notification, démarrage avec Windows) pour que le raccourci global réponde toujours,
   ou l'utilisateur la lance-t-il à la demande ? Le cas 1 (« l'erreur ne va pas rester »)
   n'a de sens que si l'application écoute déjà. Cette décision change le produit, pas
   seulement l'implémentation.
9. **Que fait-on d'une capture défilante dont le recollage est douteux ?** Refus,
   ou livraison avec un avertissement visible ? Le cas 5 propose l'avertissement ; ce
   n'est pas arbitré.
10. **Combinaisons de raccourcis par défaut**, et repli si elles sont prises. Non
    décidées à ce jour.
11. **Signature de code** de l'installeur : un certificat a un coût annuel réel pour un
    développeur solo. Sans lui, chaque installation affronte SmartScreen. Décision
    budgétaire, à prendre avant la première livraison, pas après. Chiffrée dans
    `docs/UPDATES.md` §5.2.
