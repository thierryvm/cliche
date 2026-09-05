/*
 * The system, on one page. Reachable at #/systeme.
 *
 * PURELY VISUAL, ON PURPOSE. No `invoke`, no import of ../displays: this page
 * has to open in a plain browser tab, outside the Tauri webview, or it cannot
 * be LOOKED at while it is being written. Every state below is drawn, never
 * fetched.
 *
 * The hover and pressed specimens carry `is-hover` / `is-active`, and the
 * focus specimen carries `is-focus`. Those classes are not a second style
 * sheet: they sit inside the SAME rules as :hover, :active and :focus-visible
 * in components.css, so a specimen cannot drift from the state it claims to
 * show. Disabled uses the real `disabled` attribute, not a class.
 */

import { Fragment } from 'react';
import type { ReactNode } from 'react';

import { Glyph, ICON } from './Glyph';
import './components.css';

type SpecimenProps = {
  readonly caption: string;
  readonly wide?: boolean;
  readonly children: ReactNode;
};

function Specimen({ caption, wide = false, children }: SpecimenProps) {
  return (
    <div className={wide ? 'c-specimen c-specimen--wide' : 'c-specimen'}>
      {children}
      <p className="c-caption">{caption}</p>
    </div>
  );
}

type SectionProps = {
  readonly id: string;
  readonly label: string;
  readonly children: ReactNode;
};

function Section({ id, label, children }: SectionProps) {
  return (
    <section className="c-section" aria-labelledby={id}>
      <h2 className="c-section__label" id={id}>
        {label}
      </h2>
      {children}
    </section>
  );
}

const BUTTON_VARIANTS = [
  ['primary', 'Capturer', '--accent / --text-on-solid'],
  ['secondary', 'Ouvrir', '--surface / --border-strong'],
  ['ghost', 'Annuler', 'transparent / --text'],
  ['danger', 'Supprimer', '--danger, rempli au survol'],
] as const;

const BUTTON_STATES = [
  ['', 'repos'],
  ['is-hover', 'survol'],
  ['is-active', 'actif'],
  ['is-focus', 'focus clavier'],
] as const;

/* ==================================================================== *
 * V1 SPECIMENS — the pieces that did not exist before 5 September 2026:
 * the window shell, the glass title bar, the launcher, the transient
 * confirmation, the shortcut recorder and the key map.
 *
 * Still purely visual. No `invoke`, no window API: this page has to open in
 * a plain browser tab. A window control here is a button that does nothing,
 * and it is captioned as such.
 * ==================================================================== */

const CAPTURE_KEYS = ['Ctrl', 'Maj', '2'] as const;

/* `as const` for the same reason BUTTON_STATES carries it: tsconfig sets
   noUncheckedIndexedAccess, so destructuring a plain string[] hands back
   `string | undefined`. */
const LAUNCH_STATES = [
  ['', 'repos · le seul accent de l’écran'],
  ['is-hover', 'survol · --accent-hover'],
  ['is-active', 'actif · --accent-active, aplati'],
  ['is-focus', 'focus clavier'],
] as const;

/** A combination: one chip per key, so it folds instead of crushing. */
function Keys({ keys }: { readonly keys: readonly string[] }) {
  return (
    <span className="c-keys">
      {keys.map((key, index) => (
        <Fragment key={`${key}-${String(index)}`}>
          {index > 0 && <span aria-hidden="true">+</span>}
          <span className="c-kbd">{key}</span>
        </Fragment>
      ))}
    </span>
  );
}

type WindowFrameProps = {
  readonly title: string;
  readonly narrow?: boolean;
  readonly maximised?: boolean;
  readonly overlay?: ReactNode;
  readonly children: ReactNode;
};

/**
 * The product window, at one of the two widths it is judged at.
 *
 * `data-tauri-drag-region` is written here rather than left to the screen that
 * will use it: it is HALF of the drag decision, and the other half
 * (`-webkit-app-region`) is in components.css. Splitting one behaviour across
 * two files with only one half visible is how the veil lost its handles.
 * Neither half is verified — see the report.
 */
function WindowFrame({
  title,
  narrow = false,
  maximised = false,
  overlay,
  children,
}: WindowFrameProps) {
  return (
    <div className={narrow ? 'c-frame c-frame--narrow' : 'c-frame'}>
      <div className="c-shell">
        <div className="c-titlebar" data-tauri-drag-region>
          <p className="c-titlebar__title">{title}</p>
          <div className="c-titlebar__controls">
            <button
              type="button"
              className="c-btn c-btn--ghost c-btn--icon c-winbtn"
              aria-label="Réduire"
            >
              <Glyph d={ICON.minimize} />
            </button>
            <button
              type="button"
              className="c-btn c-btn--ghost c-btn--icon c-winbtn"
              aria-label={maximised ? 'Restaurer' : 'Agrandir'}
            >
              <Glyph d={maximised ? ICON.restore : ICON.maximize} />
            </button>
            <button
              type="button"
              className="c-btn c-btn--ghost c-btn--icon c-winbtn c-winbtn--close"
              aria-label="Fermer"
            >
              <Glyph d={ICON.close} />
            </button>
          </div>
        </div>
        <div className="c-shell__body">{children}</div>
        {overlay}
      </div>
    </div>
  );
}

type ShortcutState = 'ready' | 'loading' | 'refused';

/** The home screen: three capture actions, and what the shortcut is doing. */
function LauncherBody({ shortcut }: { readonly shortcut: ShortcutState }) {
  return (
    <>
      <h3 className="c-screen__name">Capturer</h3>

      <div
        className="c-launch"
        role="group"
        aria-label="Actions de capture"
        style={{ marginBlockStart: 'var(--space-5)' }}
      >
        <button type="button" className="c-launch__item c-launch__item--primary">
          <Glyph d={ICON.capture} />
          <span className="c-launch__name">Capturer une zone</span>
        </button>
        <button
          type="button"
          className="c-launch__item c-launch__item--soon"
          aria-disabled="true"
        >
          <Glyph d={ICON.window} />
          <span className="c-launch__name">Capturer une fenêtre</span>
          <span className="c-badge">à venir</span>
        </button>
        <button
          type="button"
          className="c-launch__item c-launch__item--soon"
          aria-disabled="true"
        >
          <Glyph d={ICON.screen} />
          <span className="c-launch__name">Capturer tout l&apos;écran</span>
          <span className="c-badge">à venir</span>
        </button>
      </div>

      {shortcut === 'ready' && (
        <p className="c-hint" style={{ marginBlockStart: 'var(--space-5)' }}>
          <Keys keys={CAPTURE_KEYS} />
          <span>capture une zone, même quand Cliché est en arrière-plan.</span>
        </p>
      )}

      {shortcut === 'loading' && (
        <p className="c-hint" style={{ marginBlockStart: 'var(--space-5)' }} aria-busy="true">
          <span className="c-kbd c-skeleton">Ctrl + Maj + 2</span>
          <span>lecture du registre des raccourcis…</span>
        </p>
      )}

      {shortcut === 'refused' && (
        <div
          className="c-note c-note--danger"
          role="alert"
          style={{ marginBlockStart: 'var(--space-5)' }}
        >
          <Glyph d={ICON.alert} />
          <span>
            <strong>Raccourci refusé</strong> — <span className="c-num">Ctrl + Maj + 2</span>{' '}
            est tenu par une autre application. Cliché tourne sans son raccourci ; les trois
            actions ci-dessus restent utilisables à la souris.
          </span>
        </div>
      )}

      <p className="c-frame__filler">
        Cette ligne n&apos;existe que dans la vitrine. Sans contenu plus haut que la fenêtre,
        rien ne défile sous la barre de titre — et le verre montrerait un aplat en prétendant
        montrer du verre. Faites défiler ce cadre pour voir le matériau travailler.
      </p>
    </>
  );
}

export default function Showcase() {
  return (
    <main className="c-showcase">
      <div className="c-showcase__inner">
        <header>
          <h1 className="c-showcase__title">Cliché — le système</h1>
          <p className="c-showcase__lede">
            Chaque composant, dans chaque état, avec le jeton qu&apos;il dépense.
            Aucune valeur n&apos;est écrite ici : tout vient de{' '}
            <code>src/design/tokens.css</code>.
          </p>
        </header>

        {/* ============================================================
            V1 — les composants neufs. Placés EN TÊTE parce que c'est ce
            qui est en revue aujourd'hui ; le reste de la page n'a pas
            changé.
            ============================================================ */}

        <Section id="s-window" label="V1 · la fenêtre — barre de titre en verre, lanceur dessous">
          <div className="c-stack">
            <Specimen
              caption="480 × 600 — le minimum déclaré dans tauri.conf.json. Barre 52 px, trois commandes de 44 px, la page défile DESSOUS. CE CADRE NE VAUT 480 PX QU'À PARTIR D'UNE PAGE DE 512 (--c-frame-app + 2 × --gutter) : la vitrine dépense sa propre gouttière avant lui. Mesuré le 5 septembre 2026 — à une page de 480 le cadre tombe à 448, son lanceur à 414 px et à UNE colonne, là où la vraie fenêtre de 480 en donne 446 et DEUX."
              wide
            >
              <WindowFrame title="Cliché">
                <LauncherBody shortcut="ready" />
              </WindowFrame>
            </Specimen>

            <Specimen
              caption="375 px, fenêtre agrandie — titre coupé à l'ellipse, glyphe « restaurer », lanceur à une colonne. Cette largeur n'est PAS atteignable aujourd'hui : minWidth vaut 480. Même réserve que ci-dessus : ce cadre ne vaut 375 px qu'à partir d'une page de 407 (--c-frame-narrow + 2 × --gutter)."
              wide
            >
              <WindowFrame
                title="Cliché — un titre de fenêtre assez long pour être coupé"
                narrow
                maximised
              >
                <LauncherBody shortcut="ready" />
              </WindowFrame>
            </Specimen>
          </div>

          <ul className="c-rules" style={{ marginBlockStart: 'var(--space-4)' }}>
            <li>
              Le verre est ici parce que <strong>quelque chose bouge derrière</strong> : le
              défilement passe sous la barre. Sur un fond immobile il coûterait une couche
              composée et ne se distinguerait pas d&apos;un aplat.
            </li>
            <li>
              Deux couleurs seulement sont peintes sur ce verre : <code>--text</code> (les
              glyphes) et <code>--text-muted</code> (le nom de l&apos;application) — les deux
              seules que <code>check-contrast.mjs</code> mesure sur les trois régimes de verre.
              Aucun message, aucune touche, aucun champ n&apos;entre dans cette barre.
            </li>
            <li>
              La barre fait <code>--titlebar-height</code> = 52 px et non 44 :{' '}
              <code>--focus-ring-offset</code> dessine l&apos;anneau <em>à l&apos;extérieur</em>{' '}
              du bouton, et le bord haut de la fenêtre est le seul endroit où l&apos;extérieur
              n&apos;existe pas. À 44 px l&apos;anneau perdait son segment supérieur — pour les
              seuls utilisateurs au clavier.
            </li>
            <li>
              Aucun rayon sur la barre : Windows arrondit lui-même une fenêtre de premier
              niveau. <strong>Non vérifié</strong> sur une fenêtre sans décoration.
            </li>
          </ul>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-winbtn" label="V1 · commandes de fenêtre — 44 px, et le rouge de la plateforme">
          <div className="c-panel">
            <div className="c-specimens">
              <Specimen caption="repos">
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon c-winbtn"
                  aria-label="Réduire"
                >
                  <Glyph d={ICON.minimize} />
                </button>
              </Specimen>
              <Specimen caption="survol · --surface-selected">
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon c-winbtn is-hover"
                  aria-label="Agrandir"
                >
                  <Glyph d={ICON.maximize} />
                </button>
              </Specimen>
              <Specimen caption="actif · filet interne en currentColor">
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon c-winbtn is-active"
                  aria-label="Agrandir"
                >
                  <Glyph d={ICON.maximize} />
                </button>
              </Specimen>
              <Specimen caption="focus clavier">
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon c-winbtn is-focus"
                  aria-label="Restaurer"
                >
                  <Glyph d={ICON.restore} />
                </button>
              </Specimen>
              <Specimen caption="fermer · repos">
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon c-winbtn c-winbtn--close"
                  aria-label="Fermer"
                >
                  <Glyph d={ICON.close} />
                </button>
              </Specimen>
              <Specimen caption="fermer · survol --danger">
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon c-winbtn c-winbtn--close is-hover"
                  aria-label="Fermer"
                >
                  <Glyph d={ICON.close} />
                </button>
              </Specimen>
              <Specimen caption="fermer · actif">
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon c-winbtn c-winbtn--close is-active"
                  aria-label="Fermer"
                >
                  <Glyph d={ICON.close} />
                </button>
              </Specimen>
            </div>
            <p className="c-caption" style={{ marginBlockStart: 'var(--space-4)' }}>
              Aucun état désactivé : une commande de fenêtre indisponible est RETIRÉE, pas
              grisée — <code>--text-inert</code> est interdit sur le verre par une RULE des
              jetons. En thème sombre, le survol de « fermer » est un saumon pâle à glyphe
              noir : c&apos;est le système qui inverse ses remplissages, pas un accident.
            </p>
          </div>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-launch" label="V1 · tuiles du lanceur — une action construite, deux qui disent qu'elles ne le sont pas">
          <div className="c-launch">
            {LAUNCH_STATES.map(([stateClass, caption]) => (
              <div className="c-specimen" key={caption}>
                <button
                  type="button"
                  className={`c-launch__item c-launch__item--primary${
                    stateClass ? ` ${stateClass}` : ''
                  }`}
                >
                  <Glyph d={ICON.capture} />
                  <span className="c-launch__name">Capturer une zone</span>
                </button>
                <p className="c-caption">{caption}</p>
              </div>
            ))}

            <div className="c-specimen">
              <button
                type="button"
                className="c-launch__item c-launch__item--soon"
                aria-disabled="true"
              >
                <Glyph d={ICON.window} />
                <span className="c-launch__name">Capturer une fenêtre</span>
                <span className="c-badge">à venir</span>
              </button>
              <p className="c-caption">
                pas construite · bord tireté + --surface-inert + badge + aria-disabled
              </p>
            </div>

            <div className="c-specimen">
              <button
                type="button"
                className="c-launch__item c-launch__item--soon is-hover"
                aria-disabled="true"
              >
                <Glyph d={ICON.screen} />
                <span className="c-launch__name">Capturer tout l&apos;écran</span>
                <span className="c-badge">à venir</span>
              </button>
              <p className="c-caption">survol · aucune réponse, et c&apos;est le quatrième indice</p>
            </div>
          </div>

          <ul className="c-rules" style={{ marginBlockStart: 'var(--space-4)' }}>
            <li>
              Le libellé d&apos;une tuile « à venir » reste <code>--text-muted</code> et non{' '}
              <code>--text-inert</code> : WCAG dispense un contrôle inutilisable parce que
              personne n&apos;a besoin de le lire — ici c&apos;est <em>tout ce qu&apos;elle a à
              dire</em>. Le « pas là » est porté par quatre indices non colorés.
            </li>
            <li>
              La grille est <strong>inégale parce que le contenu l&apos;est</strong> : une action
              sur trois fonctionne. Trois tuiles de même largeur diraient le contraire.
            </li>
          </ul>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-hint" label="V1 · le rappel du raccourci — trois états d'information">
          <div className="c-panel c-stack">
            <Specimen caption="prêt · le raccourci est enregistré" wide>
              <p className="c-hint">
                <Keys keys={CAPTURE_KEYS} />
                <span>capture une zone, même quand Cliché est en arrière-plan.</span>
              </p>
            </Specimen>
            <Specimen caption="chargement · .c-kbd + .c-skeleton, la boîte est déjà réservée" wide>
              <p className="c-hint" aria-busy="true">
                <span className="c-kbd c-skeleton">Ctrl + Maj + 2</span>
                <span>lecture du registre des raccourcis…</span>
              </p>
            </Specimen>
            <Specimen caption="refusé · PRD R4 — la combinaison est NOMMÉE, et la suite est dite" wide>
              <div className="c-note c-note--danger" role="alert">
                <Glyph d={ICON.alert} />
                <span>
                  <strong>Raccourci refusé</strong> —{' '}
                  <span className="c-num">Ctrl + Maj + 2</span> est tenu par une autre
                  application. Cliché tourne sans son raccourci ; les trois actions restent
                  utilisables à la souris.
                </span>
              </div>
            </Specimen>
          </div>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-toast" label="V1 · la confirmation qui s'efface — et l'échec qui ne s'efface pas">
          <div className="c-stack">
            <Specimen
              caption="succès · role=status, s'efface après --dur-toast-dwell (2,4 s). Dessiné SANS .c-toast--transient : la variante réelle disparaîtrait avant qu'on l'ait regardée."
              wide
            >
              <WindowFrame
                title="Cliché"
                overlay={
                  <div className="c-toast-region">
                    <div className="c-note c-note--success c-toast" role="status">
                      <Glyph d={ICON.check} />
                      <span className="c-toast__message">
                        <span className="c-num">933×577</span> copié
                      </span>
                    </div>
                  </div>
                }
              >
                <LauncherBody shortcut="ready" />
              </WindowFrame>
            </Specimen>

            <Specimen
              caption="échec · role=alert, PAS d'effacement automatique, un bouton de 44 px pour le renvoyer. À 375 px le message se replie, il n'est jamais coupé."
              wide
            >
              <WindowFrame
                title="Cliché"
                narrow
                overlay={
                  <div className="c-toast-region">
                    <div className="c-note c-note--danger c-toast" role="alert">
                      <Glyph d={ICON.alert} />
                      <span className="c-toast__message">
                        <strong>Échec</strong> — l&apos;image n&apos;a pas atteint le
                        presse-papiers : une autre application le tient ouvert. Réessayez dans
                        un instant.
                      </span>
                      <button
                        type="button"
                        className="c-btn c-btn--ghost c-btn--icon c-toast__dismiss"
                        aria-label="Fermer le message"
                      >
                        <Glyph d={ICON.close} />
                      </button>
                    </div>
                  </div>
                }
              >
                <LauncherBody shortcut="ready" />
              </WindowFrame>
            </Specimen>
          </div>

          <ul className="c-rules" style={{ marginBlockStart: 'var(--space-4)' }}>
            <li>
              Le balisage est <code>c-note c-note--success c-toast</code> :{' '}
              <strong>aucune couleur neuve</strong>. Le composant ajoute quatre choses — où il
              se pose, jusqu&apos;où il s&apos;élargit, qu&apos;il est surélevé, et quand il
              part.
            </li>
            <li>
              <strong>Pas de verre ici</strong>, et c&apos;est une mesure et non un goût : un
              toast porte <code>--success</code> ou <code>--danger</code> comme TEXTE, or ces
              deux-là sont mesurés sur les cinq surfaces opaques et sur aucun des trois
              régimes de verre.
            </li>
            <li>
              <code>--dur-toast-dwell</code> est écrit en littéral dans les jetons :{' '}
              <code>calc(var(--dur-medium) * 10)</code> tomberait à 0 s sous{' '}
              <code>prefers-reduced-motion</code> — la confirmation disparaîtrait avant
              d&apos;être lue, précisément pour ceux qui ont demandé moins de mouvement.
            </li>
          </ul>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-recorder" label="V1 · réglages — le raccourci de capture, réglable">
          <div className="c-panel">
            <div className="c-specimens">
              <Specimen caption="repos · la combinaison en place">
                <div className="c-field">
                  <span className="c-field__label">Raccourci de capture</span>
                  <button type="button" className="c-btn c-btn--secondary c-recorder">
                    <Keys keys={CAPTURE_KEYS} />
                    <span>Modifier</span>
                  </button>
                </div>
              </Specimen>

              <Specimen caption="à l'écoute · aria-pressed + barre --accent (état actif)">
                <div className="c-field">
                  <span className="c-field__label">Raccourci de capture</span>
                  <button
                    type="button"
                    className="c-btn c-btn--secondary c-recorder"
                    aria-pressed="true"
                  >
                    <span>Appuyez sur une combinaison…</span>
                  </button>
                  <span className="c-field__message">Échap annule.</span>
                </div>
              </Specimen>

              <Specimen caption="focus clavier">
                <div className="c-field">
                  <span className="c-field__label">Raccourci de capture</span>
                  <button type="button" className="c-btn c-btn--secondary c-recorder is-focus">
                    <Keys keys={CAPTURE_KEYS} />
                    <span>Modifier</span>
                  </button>
                </div>
              </Specimen>

              <Specimen caption="conflit · aria-invalid + message nommé (A4)">
                <div className="c-field">
                  <span className="c-field__label">Raccourci de capture</span>
                  <button
                    type="button"
                    className="c-btn c-btn--secondary c-recorder"
                    aria-invalid="true"
                    aria-describedby="rec-err"
                  >
                    <Keys keys={['Ctrl', 'Maj', 'A']} />
                    <span>Modifier</span>
                  </button>
                  <span className="c-field__message c-field__message--error" id="rec-err">
                    <Glyph d={ICON.alert} />
                    Ctrl + Maj + A est déjà pris par une autre application. Essayez une autre
                    combinaison.
                  </span>
                </div>
              </Specimen>

              <Specimen caption="indisponible · le registre n'a pas pu être lu">
                <div className="c-field">
                  <span className="c-field__label">Raccourci de capture</span>
                  <button type="button" className="c-btn c-btn--secondary c-recorder" disabled>
                    <span>Indisponible</span>
                  </button>
                  <span className="c-field__message">
                    Le registre des raccourcis n&apos;a pas pu être lu.
                  </span>
                </div>
              </Specimen>

              <Specimen caption="chargement">
                <div className="c-field">
                  <span className="c-field__label">Raccourci de capture</span>
                  <button
                    type="button"
                    className="c-btn c-btn--secondary c-recorder"
                    aria-busy="true"
                  >
                    <span className="c-kbd c-skeleton">Ctrl + Maj + 2</span>
                    <span>Modifier</span>
                  </button>
                </div>
              </Specimen>
            </div>

            <p className="c-caption" style={{ marginBlockStart: 'var(--space-4)' }}>
              L&apos;état « à l&apos;écoute » ne coûte aucune règle neuve :{' '}
              <code>.c-btn[aria-pressed=&apos;true&apos;]</code> existait déjà pour l&apos;outil
              actif d&apos;une barre, et « actif » est l&apos;un des quatre emplois de
              l&apos;accent. <code>--c-recorder-min</code> empêche la boîte de grandir sous le
              pointeur à l&apos;instant où elle est armée.
            </p>
          </div>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-keymap" label="V1 · aide — le registre, jamais recopié">
          <div className="c-panel">
            <h3 className="c-section__label">Capture</h3>
            <dl className="c-keymap">
              <dt>
                <Keys keys={CAPTURE_KEYS} />
              </dt>
              <dd>Capturer une zone</dd>

              <dt>
                <Keys keys={['Échap']} />
              </dt>
              <dd>Fermer le voile sans capturer</dd>

              <dt>
                <Keys keys={['Ctrl', 'Maj', '3']} />
              </dt>
              <dd>Capturer la fenêtre sous le pointeur</dd>
              <dd className="c-keymap__note c-field__message c-field__message--error">
                <Glyph d={ICON.alert} />
                Refusé par Windows — une autre application tient cette combinaison. Réglez-en
                une autre dans Réglages.
              </dd>

              <dt>
                <Keys keys={['Ctrl', 'Maj', 'Alt', 'Windows', 'F12']} />
              </dt>
              <dd>
                Une action dont le nom est délibérément long, pour montrer que la colonne de
                droite se replie et que celle de gauche plie ses touches au lieu de les écraser
              </dd>
            </dl>
          </div>

          <ul className="c-rules" style={{ marginBlockStart: 'var(--space-4)' }}>
            <li>
              C&apos;est un <code>&lt;dl&gt;</code> et la <strong>combinaison est le terme</strong> :
              ce qu&apos;on cherche dans une aide, c&apos;est la touche qu&apos;on vient
              d&apos;appuyer. Cela colle aussi les deux colonnes l&apos;une à l&apos;autre — une
              colonne de noms en <code>1fr</code> avec les touches rejetées à droite devient
              illisible dès que la fenêtre s&apos;élargit.
            </li>
            <li>
              Volontairement pas construit sur <code>.c-row</code> : cette ligne-là s&apos;allume
              au survol et prend un curseur de main, ce qui promet un clic qui ne fait rien ici.
            </li>
            <li>
              Une combinaison refusée s&apos;explique sur sa propre ligne, en travers des deux
              colonnes — PRD R4 : jamais d&apos;échec silencieux, et la combinaison est nommée.
            </li>
          </ul>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-type" label="Typographie — quatre tailles, rapport 1,25">
          <div className="c-panel c-stack">
            <Specimen caption="--size-title · --font-display · une par fenêtre" wide>
              <p
                className="c-scale__sample"
                style={{ fontSize: 'var(--size-title)', fontFamily: 'var(--font-display)' }}
              >
                Cliché
              </p>
            </Specimen>
            <Specimen caption="--size-section · un titre de section, ou un chiffre montré" wide>
              <p className="c-scale__sample" style={{ fontSize: 'var(--size-section)' }}>
                Capture 3 sur 12
              </p>
            </Specimen>
            <Specimen caption="--size-body · tout ce qu'on lit pour agir" wide>
              <p className="c-scale__sample" style={{ fontSize: 'var(--size-body)' }}>
                Enregistrer la sélection dans la bibliothèque
              </p>
            </Specimen>
            <Specimen caption="--size-caption · --font-numeric · faits en pixels, unités, horodatages" wide>
              <p
                className="c-scale__sample"
                style={{
                  fontSize: 'var(--size-caption)',
                  fontFamily: 'var(--font-numeric)',
                  color: 'var(--text-muted)',
                }}
              >
                1920×1080 px · échelle 1.5 · 14:32:07
              </p>
            </Specimen>
          </div>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-buttons" label="Boutons — quatre variantes, cinq états">
          <div className="c-panel c-stack">
            {BUTTON_VARIANTS.map(([variant, label, note]) => (
              <div key={variant}>
                <p className="c-caption">
                  .c-btn--{variant} — {note}
                </p>
                <div className="c-specimens" style={{ marginBlockStart: 'var(--space-3)' }}>
                  {BUTTON_STATES.map(([stateClass, stateLabel]) => (
                    <Specimen key={stateLabel} caption={stateLabel}>
                      <button
                        type="button"
                        className={`c-btn c-btn--${variant}${stateClass ? ` ${stateClass}` : ''}`}
                      >
                        {label}
                      </button>
                    </Specimen>
                  ))}
                  <Specimen caption="désactivé · bord tireté (A4)">
                    <button type="button" className={`c-btn c-btn--${variant}`} disabled>
                      {label}
                    </button>
                  </Specimen>
                </div>
              </div>
            ))}

            <p className="c-caption">
              Le focus est dessiné avec --focus-ring-offset : il atterrit sur la
              surface, jamais sur l&apos;accent. Tabulez dans la page pour le voir
              en vrai — la classe de démonstration peint la même règle.
            </p>
          </div>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-icons" label="Bouton icône — 44 px, c'est une RULE">
          <div className="c-panel c-stack">
            <div className="c-specimens">
              <Specimen caption="repos · --hit-min">
                <button type="button" className="c-btn c-btn--ghost c-btn--icon" aria-label="Capturer">
                  <Glyph d={ICON.capture} />
                </button>
              </Specimen>
              <Specimen caption="survol">
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon is-hover"
                  aria-label="Annoter"
                >
                  <Glyph d={ICON.pen} />
                </button>
              </Specimen>
              <Specimen caption="focus clavier">
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon is-focus"
                  aria-label="Masquer"
                >
                  <Glyph d={ICON.mask} />
                </button>
              </Specimen>
              <Specimen caption="outil actif · aria-pressed + barre">
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon"
                  aria-pressed="true"
                  aria-label="Copier"
                >
                  <Glyph d={ICON.copy} />
                </button>
              </Specimen>
              <Specimen caption="désactivé · --surface-inert">
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon"
                  aria-label="Supprimer"
                  disabled
                >
                  <Glyph d={ICON.trash} />
                </button>
              </Specimen>
            </div>
          </div>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-fields" label="Champs de saisie">
          <div className="c-panel c-stack">
            <div className="c-specimens">
              <Specimen caption="repos · --border-strong (jamais --border)">
                <div className="c-field">
                  <label className="c-field__label" htmlFor="f-rest">
                    Nom du fichier
                  </label>
                  <input
                    id="f-rest"
                    className="c-input"
                    type="text"
                    defaultValue="capture-2026-09-03"
                  />
                </div>
              </Specimen>

              <Specimen caption="focus · bord --accent + anneau">
                <div className="c-field">
                  <label className="c-field__label" htmlFor="f-focus">
                    Dossier
                  </label>
                  <input id="f-focus" className="c-input is-focus" type="text" defaultValue="Captures" />
                </div>
              </Specimen>

              <Specimen caption="désactivé · bord tireté + --text-inert">
                <div className="c-field">
                  <label className="c-field__label" htmlFor="f-off">
                    Rétention
                  </label>
                  <input id="f-off" className="c-input" type="text" defaultValue="30 jours" disabled />
                  <span className="c-field__message">Réglable quand la bibliothèque est active.</span>
                </div>
              </Specimen>

              <Specimen caption="erreur · aria-invalid + message nommé">
                <div className="c-field">
                  <label className="c-field__label" htmlFor="f-err">
                    Raccourci
                  </label>
                  <input
                    id="f-err"
                    className="c-input"
                    type="text"
                    defaultValue="Ctrl+Maj+A"
                    aria-invalid="true"
                    aria-describedby="f-err-msg"
                  />
                  <span className="c-field__message c-field__message--error" id="f-err-msg">
                    <Glyph d={ICON.alert} />
                    Raccourci déjà pris par Windows.
                  </span>
                </div>
              </Specimen>

              <Specimen caption="numérique · --font-numeric, chiffres alignés">
                <div className="c-field">
                  <label className="c-field__label" htmlFor="f-num">
                    Largeur
                  </label>
                  <input
                    id="f-num"
                    className="c-input c-input--numeric"
                    type="text"
                    defaultValue="1920"
                  />
                </div>
              </Specimen>

              <Specimen caption="vide · le premier jour, --text-muted">
                <div className="c-field">
                  <label className="c-field__label" htmlFor="f-empty">
                    Préfixe
                  </label>
                  <input id="f-empty" className="c-input" type="text" placeholder="cliché-" />
                </div>
              </Specimen>
            </div>
          </div>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-glass" label="Verre — sur les deux fonds extrêmes">
          <div className="c-band">
            <div className="c-band__half c-band__half--dark" />
            <div className="c-band__half c-band__half--light" />
            <div className="c-band__pinstripe" />
            <div className="c-glass">
              <h3 className="c-glass__title">Sélection</h3>
              <p className="c-glass__facts">1204×780 px · échelle 1.5 · PNG</p>
              <div
                className="c-toolbar"
                style={{ marginBlockStart: 'var(--space-3)', padding: 0 }}
                role="toolbar"
                aria-label="Outils de capture"
              >
                <button type="button" className="c-btn c-btn--ghost c-btn--icon" aria-label="Recadrer">
                  <Glyph d={ICON.capture} />
                </button>
                <button
                  type="button"
                  className="c-btn c-btn--ghost c-btn--icon"
                  aria-pressed="true"
                  aria-label="Annoter"
                >
                  <Glyph d={ICON.pen} />
                </button>
                <button type="button" className="c-btn c-btn--ghost c-btn--icon" aria-label="Masquer">
                  <Glyph d={ICON.mask} />
                </button>
                <button type="button" className="c-btn c-btn--ghost c-btn--icon" aria-label="Copier">
                  <Glyph d={ICON.copy} />
                </button>
                <button type="button" className="c-btn c-btn--primary" style={{ minWidth: 'var(--hit-min)' }}>
                  Copier
                </button>
              </div>
            </div>
            <p className="c-band__caption">moitié noire · moitié blanche</p>
          </div>

          <ul className="c-rules" style={{ marginBlockStart: 'var(--space-4)' }}>
            <li>
              Le cas qui contraint est le fond <strong>blanc en thème sombre</strong> :
              le plancher d&apos;alpha y est 0,787 pour 0,80 déclaré, soit 0,013 de
              marge. La valeur vient de <code>--glass-alpha</code> dans{' '}
              <code>tokens.css</code>, et <code>scripts/check-contrast.mjs</code> la
              recalcule à chaque <code>pnpm test</code> et en CI : baisser l&apos;alpha
              sous ce plancher fait échouer la vérification, pas seulement avertir.
            </li>
            <li>
              <code>--text-inert</code> est interdit sur verre (RULE) : aucun contrôle
              désactivé n&apos;est posé dans ce panneau.
            </li>
            <li>
              Un seul panneau de verre sur cette page. La RULE en autorise trois par
              écran, jamais imbriqués, jamais dans une liste.
            </li>
          </ul>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-rows" label="Lignes de liste">
          <div className="c-panel">
            {/* role=listbox/option, not plain buttons: aria-selected only means
                something inside a selection widget, and the CSS keys on it. */}
            <ul className="c-list" role="listbox" aria-label="Captures récentes">
              <li role="presentation">
                <button type="button" role="option" aria-selected="false" className="c-row">
                  <Glyph d={ICON.copy} />
                  <span className="c-row__name">capture-2026-09-03-143207.png</span>
                  <span className="c-row__facts">1920×1080</span>
                </button>
              </li>
              <li role="presentation">
                <button type="button" role="option" aria-selected="false" className="c-row is-hover">
                  <Glyph d={ICON.copy} />
                  <span className="c-row__name">écran-de-connexion-client.png</span>
                  <span className="c-row__facts">1204×780</span>
                </button>
              </li>
              <li role="presentation">
                <button type="button" role="option" className="c-row" aria-selected="true">
                  <Glyph d={ICON.check} />
                  <span className="c-row__name">rapport-trimestriel-page-4.png</span>
                  <span className="c-row__facts">2560×1440</span>
                </button>
              </li>
              <li role="presentation">
                <button type="button" role="option" aria-selected="false" className="c-row is-focus">
                  <Glyph d={ICON.copy} />
                  <span className="c-row__name">
                    un-nom-de-fichier-beaucoup-trop-long-pour-la-fenêtre-de-480-pixels.png
                  </span>
                  <span className="c-row__facts">800×600</span>
                </button>
              </li>
            </ul>
          </div>
          <p className="c-caption" style={{ marginBlockStart: 'var(--space-3)' }}>
            séparateur --border (décoratif) · sélection --surface-selected + barre
            --accent + coche · débordement coupé à l&apos;ellipse
          </p>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-grid" label="Bibliothèque — 2 colonnes à 480 px (2×212 + 24 = 448)">
          <ul className="c-grid" role="listbox" aria-label="Bibliothèque">
            <li role="presentation">
              <button type="button" role="option" aria-selected="false" className="c-tile">
                <span className="c-tile__frame" />
                <span className="c-tile__name">capture-2026-09-03.png</span>
                <span className="c-tile__facts">1920×1080 · 412 Ko</span>
              </button>
            </li>
            <li role="presentation">
              <button type="button" role="option" className="c-tile" aria-selected="true">
                <span className="c-tile__frame">
                  <span className="c-tile__badge">
                    <Glyph d={ICON.check} />
                  </span>
                </span>
                <span className="c-tile__name">écran-de-connexion.png</span>
                <span className="c-tile__facts">1204×780 · 208 Ko</span>
              </button>
            </li>
            <li role="presentation">
              <button type="button" role="option" aria-selected="false" className="c-tile is-focus">
                <span className="c-tile__frame" />
                <span className="c-tile__name">
                  un-nom-de-fichier-beaucoup-trop-long-pour-tenir.png
                </span>
                <span className="c-tile__facts">2560×1440 · 1,2 Mo</span>
              </button>
            </li>
            <li role="presentation">
              {/* The placeholder wears the SAME classes as the tile it stands
                  in for — __frame, __name, __facts — so it reserves the exact
                  box the loaded tile will take, by construction and not by a
                  re-typed height. It used to skip the __name line, and the
                  whole row jumped up when a thumbnail arrived. The word stays
                  in the markup for a screen reader; .c-skeleton paints it out. */}
              <span className="c-tile" aria-busy="true">
                <span className="c-tile__frame c-skeleton" />
                <span className="c-tile__name c-skeleton" aria-hidden="true">
                  &nbsp;
                </span>
                <span className="c-tile__facts c-skeleton">chargement…</span>
              </span>
            </li>
          </ul>

          <div className="c-stack" style={{ marginBlockStart: 'var(--space-5)' }}>
            <div className="c-empty">
              <Glyph d={ICON.capture} />
              <h3 className="c-empty__title">Aucune capture</h3>
              <p style={{ margin: 0 }}>
                Ctrl + Maj + 2 découpe une zone. Rien ne quitte cette machine.
              </p>
              <button type="button" className="c-btn c-btn--primary">
                Capturer une zone
              </button>
            </div>
            <p className="c-caption">état vide · --space-7, le seul endroit qui le dépense</p>
          </div>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-notes" label="Messages — l'emploi, jamais l'émotion">
          <div className="c-stack">
            <div className="c-note c-note--danger" role="alert">
              <Glyph d={ICON.alert} />
              <span>
                <strong>Échec</strong> — le fichier n&apos;a pas pu être écrit :
                dossier en lecture seule.
              </span>
            </div>
            <div className="c-note c-note--warning">
              <Glyph d={ICON.info} />
              <span>
                <strong>Irréversible</strong> — appliquer le masque grave les pixels
                dans le fichier livré.
              </span>
            </div>
            <div className="c-note c-note--success" role="status">
              <Glyph d={ICON.check} />
              <span>
                <strong>Copié</strong> — l&apos;image est dans le presse-papiers.
              </span>
            </div>
            <p className="c-caption">
              --danger / --warning / --success, chacun doublé d&apos;un mot et d&apos;une
              icône (PRD A4)
            </p>
          </div>
        </Section>

        {/* ---------------------------------------------------------- */}
        <Section id="s-surfaces" label="Surfaces et lignes">
          <div className="c-panel">
            <div className="c-swatches">
              {[
                ['--bg-window', 'fenêtre, sans Mica'],
                ['--surface', 'le plan par défaut'],
                ['--surface-raised', 'menus, popovers'],
                ['--surface-inert', 'contrôle inutilisable'],
                ['--surface-selected', 'la sélection'],
                ['--accent', 'action, actif, focus'],
                ['--border-strong', 'bord de contrôle'],
                ['--border', 'séparateur décoratif'],
              ].map(([token, use]) => (
                <div className="c-swatch" key={token}>
                  <span className="c-swatch__chip" style={{ backgroundColor: `var(${token})` }} />
                  <span className="c-caption">{token}</span>
                  <span className="c-caption">{use}</span>
                </div>
              ))}
            </div>
          </div>
        </Section>

        <Section id="s-rules" label="Ce que ce matériau s'interdit">
          <ul className="c-rules">
            <li>
              <code>--border</code> ne borde jamais un contrôle — uniquement deux
              lignes de même nature. Il mesure 1,33:1 sur <code>--surface</code>, et
              de 1,01:1 à 1,52:1 sur les cinq surfaces des deux thèmes : loin des
              3:1 qu&apos;exige WCAG 1.4.11 pour une limite de contrôle. Chiffres
              recalculés depuis <code>tokens.css</code> par{' '}
              <code>scripts/check-contrast.mjs</code> à chaque <code>pnpm test</code> — 143
              paires au 5 septembre 2026, et c&apos;est le script qui imprime ce nombre, pas
              cette page.
            </li>
            <li>
              <code>--text-inert</code> ne va jamais sur le verre.
            </li>
            <li>
              Un bouton ne porte que <code>--elev-0</code> ou <code>--elev-1</code> :
              au-delà, c&apos;est un plan surélevé, pas un contrôle.
            </li>
            <li>
              L&apos;accent ne sert qu&apos;à quatre choses : action principale, état
              actif, anneau de focus, poignées de sélection.
            </li>
            <li>
              Aucun <code>@media</code> dans <code>components.css</code> : thème sombre
              et replis du verre sont déjà dans les jetons.
            </li>
          </ul>
        </Section>
      </div>
    </main>
  );
}
