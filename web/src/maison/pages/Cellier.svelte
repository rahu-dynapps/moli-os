<script>
  import { onMount } from 'svelte';
  import Icon from '../ui/Icon.svelte';
  import Liste from '../ui/cellier/Liste.svelte';
  import Panier from '../ui/cellier/Panier.svelte';
  import Produits from '../ui/cellier/Produits.svelte';
  import Reglages from '../ui/cellier/Reglages.svelte';
  import { cellier, loadCellier } from '../lib/cellier.svelte.js';
  import { t } from '../../lib/i18n.svelte.js';

  /** « Le Cellier »: the household's shopping list and the drives' basket.
   *  Humans add needs and choose; an agent reads prices and proposes baskets
   *  in the background, so everything here follows `pantry_changed`. */
  let { route = '' } = $props();

  const TABS = ['liste', 'panier', 'produits', 'reglages'];
  const tab = $derived(TABS.includes(route.split('/')[1]) ? route.split('/')[1] : 'liste');

  onMount(loadCellier);

  // Small cues on the tabs: what is on the list, a basket waiting for a
  // choice, sheets an agent found that nobody checked yet.
  const pending = $derived(cellier.offers.filter((o) => o.status === 'pending').length);
  const badge = $derived({
    liste: cellier.items.length || null,
    panier: cellier.basket?.status === 'proposed' ? '•' : null,
    produits: pending || null,
    reglages: null,
  });
</script>

<div class="cellier">
  <header>
    <h1 class="page-title">{t('cellier.titre')}</h1>
    <p class="page-sub">{t('cellier.sous_titre')}</p>
  </header>

  {#if !cellier.loaded}
    <p class="muted">…</p>
  {:else if cellier.absent}
    <section class="card absent">
      <span class="ic"><Icon name="basket" size={26} /></span>
      <div>
        <h2>{t('cellier.absent.titre')}</h2>
        <p class="muted">{t('cellier.absent.texte')}</p>
      </div>
    </section>
  {:else}
    {#if cellier.error}<p class="error" role="alert"><Icon name="alert" size={18} />{cellier.error}</p>{/if}

    <nav class="tabs" aria-label={t('cellier.vues')}>
      {#each TABS as id (id)}
        <a href="#/cellier/{id}" class:on={tab === id} aria-current={tab === id ? 'page' : undefined}>
          {t(`cellier.onglet.${id}`)}
          {#if badge[id]}<small class="num">{badge[id]}</small>{/if}
        </a>
      {/each}
    </nav>

    {#if tab === 'panier'}
      <Panier />
    {:else if tab === 'produits'}
      <Produits />
    {:else if tab === 'reglages'}
      <Reglages />
    {:else}
      <Liste />
    {/if}
  {/if}
</div>

<style>
  .cellier {
    display: grid;
    gap: 20px;
    max-width: 980px;
  }

  .absent {
    display: flex;
    gap: 16px;
    align-items: flex-start;
  }

  .absent h2 {
    font-size: 18px;
    font-weight: 650;
    margin-bottom: 4px;
  }

  .ic {
    width: 48px;
    height: 48px;
    flex: none;
    border-radius: 16px;
    display: grid;
    place-items: center;
    background: var(--warm-soft);
    color: var(--warm-ink);
  }

  .error {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    border-radius: var(--r-sm);
    background: var(--alert-soft);
    color: var(--alert);
    font-weight: 550;
  }

  .tabs {
    display: flex;
    gap: 2px;
    padding: 4px;
    border-radius: 999px;
    background: var(--surface-3);
    justify-self: start;
    max-width: 100%;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .tabs a {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 40px;
    padding: 0 16px;
    border-radius: 999px;
    color: var(--ink-3);
    font-size: 14.5px;
    font-weight: 650;
    text-decoration: none;
    white-space: nowrap;
    transition: background 0.2s var(--ease), color 0.2s var(--ease);
  }

  .tabs a.on {
    background: var(--surface);
    color: var(--ink);
    box-shadow: var(--shadow);
  }

  .tabs small {
    min-width: 20px;
    height: 20px;
    padding: 0 6px;
    border-radius: 999px;
    display: inline-grid;
    place-items: center;
    background: var(--warm-soft);
    color: var(--warm-ink);
    font-size: 12px;
    font-weight: 700;
  }

  @media (max-width: 760px) {
    .tabs {
      justify-self: stretch;
    }

    .tabs a {
      flex: 1;
      justify-content: center;
      padding: 0 10px;
    }
  }
</style>
