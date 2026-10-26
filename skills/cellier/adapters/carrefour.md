# Adaptateur Carrefour (`carrefour`)

> **Statut : brouillon, pas encore vérifié sur un magasin réglé.** Observé sans compte connecté ni magasin choisi : les
> prix affichés sont alors des prix par défaut, pas ceux du drive. Les sections marquées « à
> observer » restent à remplir dans une session connectée, magasin choisi, puis la vérification
> (3 prix, ajouter / retirer 1 article) à faire.

- Site : `https://www.carrefour.fr/`
- Genre : drive ou livraison selon le mode choisi ; le Cellier n'utilise que le **drive**.

## Le magasin

- L'en-tête porte une recherche (« Pain, lait, oeufs... »), « Aide & contact », « Mes produits »
  (`/courses-en-5-min`), « Mon compte » et « Voir le panier ».
- « Mon compte » ouvre un menu : « Me connecter » / « Créer mon compte » en tête = pas de session.
  S'arrêter et demander à l'humain de se connecter.
- Sans magasin choisi, l'accueil propose « Faire mes courses » (bloc « Courses du quotidien ») et un
  bouton « Drive » (« Retrait gratuit en magasin ») : c'est le choix du mode et du magasin,
  **réservé à l'humain**.
- Avec un magasin choisi : *à observer* (où l'en-tête affiche le magasin et le mode). Vérifier que le
  mode est bien **Drive** et que le magasin est celui de `settings.store`.

## Chercher

- URL de recherche : `https://www.carrefour.fr/s?q=<termes>` (`+` pour l'espace).
- `get_page_text` sur les résultats ne rend que le premier produit ; lister les liens des titres avec
  `find` (« liens des titres de produits ») puis `read_page` sur un lien pour son `href`.
- Les résultats mélangent des produits voisins (lait infantile, café…) : vérifier le libellé.

## La fiche produit

- URL : `https://www.carrefour.fr/p/<slug>-<EAN>`. L'EAN (13 chiffres en fin d'URL) identifie le
  produit : c'est la `reference` (et l'`ean`) de la fiche dans Moli.
- `get_page_text` rend, vers la fin, un bloc « Drive ou Livraison » : le prix en deux morceaux
  (`6` puis `,00` puis `€`, à recoller), puis le prix unitaire sous la forme `1.00 € / L` (point
  décimal, unité après la barre), puis la promo éventuelle (« Ouvrir description de la promotion »
  suivi du libellé, par exemple « LE 2ÈME À -20% »), puis la mention « Le prix de ce produit dépend
  de votre mode de retrait (Drive ou Livraison) et de votre magasin. »
- Le conditionnement est sous le titre (« le pack de 6 briques d'1L ») : il donne `unit_qty` et
  `unit`.
- Rupture : *à observer* (libellé affiché à la place du bouton d'achat).
- Le titre de l'onglet reprend le libellé et le conditionnement.

## Le panier

- URL du panier : le lien « Voir le panier » de l'en-tête (`/cart/...`, le chemin dépend du mode) ;
  *à confirmer* une fois le magasin choisi.
- Ajouter une quantité : *à observer* (bouton « Acheter » sur les résultats et la fiche, puis
  sélecteur de quantité).
- Vider le panier : *à observer*.

## La limite à ne pas franchir

*À observer* : le bouton du panier qui mène au choix du créneau et au paiement. L'agent s'arrête au
panier rempli, avant ce bouton.

## Pièges vus en vrai

- Bandeau de cookies à la première visite : « Refuser tous les cookies » le ferme sans
  consentement.
- Sans magasin, des prix s'affichent quand même : ce ne sont **pas** ceux du drive. Ne jamais les
  relever.
- Un assistant de courses (« Demander à hopla ») flotte en bas de page : ne pas s'en servir.
- Promotions de type « le 2e à -x % » : noter dans `promo`, proposer en écart, ne pas doubler la
  quantité soi-même.
