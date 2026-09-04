# Mesure de référence avant le correctif des clignotements

**Écrit le 4 septembre 2026.** Courte : environ **5 minutes**, dont 40 secondes
pendant lesquelles ton écran clignote 20 fois.

## Pourquoi maintenant, et pas après le correctif

Ta mesure du matin — **125,7 ms de médiane, 140,9 de p95** — a été prise sur un
`main` qui n'avait **ni les poignées ni les deux correctifs presse-papier**.
`main` a bougé trois fois depuis. Comparer le futur correctif à ce chiffre-là
reviendrait à mesurer deux changements et à en attribuer le coût à un seul.

Et le nombre qui compte n'est plus la médiane : **la marge au p95 est de
9,1 ms** (140,9 contre un budget de 150). C'est le chiffre le plus serré du
projet. Il mérite d'être connu sur le code d'aujourd'hui avant qu'on empile
quoi que ce soit dessus.

---

## Étape 1 — Se placer sur le vrai `main`

```powershell
cd F:\PROJECTS\Apps\cliche
git checkout main
git pull --ff-only
git log --oneline -3
```

Tu dois voir, dans cet ordre :

```
6c55ae9  fix(veil): keep the frozen frame until the copy has actually worked
26af797  test(veil): put the zone model under test, and stop losing the clipboard failure
ca0d5bd  feat(veil): make the eight grips real, and measure the colour they needed
```

Si ce n'est pas ça, arrête-toi et dis-le-moi : la mesure porterait sur autre
chose que ce qu'on croit.

## Étape 2 — Les 20 mesures

```powershell
$env:CLICHE_BENCH = '20'; $env:CLICHE_TRANSPORT = 'bmp'; pnpm tauri dev
```

**Ce qui se passe :** 30 à 60 secondes de compilation, la fenêtre « Cliché »
s'ouvre, puis **ton écran est recouvert 20 fois de suite**, environ une fois par
seconde. Ne touche à rien pendant ces quarante secondes.

**À lire dans le terminal**, le dernier bloc :

```
[cliche] bench: finished
[cliche] bench: transport A - custom protocol, BMP (header + memcpy)
[cliche] timing report over 20 run(s)
[cliche]   #1 capture     median  XX.X ms  p95  XX.X ms
[cliche]   #2 transport   median   X.X ms  p95   X.X ms
[cliche]   #3 shown       median   X.X ms  p95   X.X ms
[cliche]   #4 painted     median  XX.X ms  p95  XX.X ms
[cliche]   TOTAL        median XXX.X ms  p95 XXX.X ms
```

**Copie le bloc entier.** Les quatre étapes comptent autant que le total : c'est
`painted` qui portera le correctif, et `shown` qui dira si l'inversion coûte
quelque chose.

## Étape 3 — Regarder les deux clignotements sur le code d'AUJOURD'HUI

`main` a changé depuis ce matin. Ils sont peut-être différents.

**Ne ferme pas l'application.**

1. `Ctrl` + `Maj` + `2`, puis **Échap**. Est-ce que l'écran « s'allume deux
   fois » comme ce matin, ou est-ce que ça a changé ?
2. `Ctrl` + `Maj` + `2`, puis **un simple clic sans glisser**. Est-ce que ça
   fait toujours « comme une photo » ? Et le terminal dit-il toujours seulement
   `veil: dismissed` ?
3. **Nouveau, à cause des correctifs presse-papier** : `Ctrl` + `Maj` + `2`,
   trace une vraie sélection, `Entrée`. Le voile doit maintenant se fermer
   **après** la copie, pas avant. Est-ce que la fermeture te paraît plus tardive
   qu'avant ? Elle devrait coûter ~10 ms, donc être invisible — si tu la
   remarques, c'est une information.

## Pour finir

`Ctrl` + `C` dans le terminal, puis :

```powershell
Remove-Item Env:\CLICHE_BENCH, Env:\CLICHE_TRANSPORT
```

Sans ça, le prochain `pnpm tauri dev` **dans ce même terminal** relancerait 20
clignotements.

---

## Ce que je veux que tu me rendes

1. Le **bloc de mesure entier** de l'étape 2.
2. Les **trois observations** de l'étape 3.

Avec ça, le correctif que tu as choisi — *n'afficher le voile qu'une fois
l'image décodée* — devient mesurable : on saura ce qu'il coûte, au lieu de
l'espérer.

---

## Ce que ce correctif changera, et ce qu'il faudra surveiller

L'ordre actuel est : `show()` **puis** on dit à la page de charger l'image
(`veil.rs`, lignes 387 et 407), et la peinture arrive ~100 ms plus tard. Pendant
ces ~100 ms, **la fenêtre est visible en affichant ce que la page avait avant** —
l'écran précédent, ou rien au tout premier déclenchement. C'est la cause unique
des deux clignotements, vérifiée dans le code.

Le correctif inverse : la page décode, accuse réception, **puis** Rust montre la
fenêtre. Le voile apparaît complet.

**Ce qu'il faudra regarder dans les chiffres d'après :**

- **`shown`** mesure 0,0 ms aujourd'hui. Après inversion il portera l'attente du
  décodage, donc il grossira — c'est attendu, ce n'est pas une régression.
- **`TOTAL`** est le seul qui compte pour le budget. Si l'inversion ne fait que
  déplacer du temps de `painted` vers `shown`, le total ne bouge pas.
- **La sensation.** Aujourd'hui tu vois quelque chose (de périmé) à ~25 ms ;
  après, tu ne verras rien jusqu'à ~125 ms puis le voile complet. **Le chiffre
  peut être identique et la sensation plus lente.** C'est toi qui trancheras, et
  aucune mesure ne le fera à ta place.
