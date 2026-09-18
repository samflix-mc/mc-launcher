# Les fonds du launcher

Trois images, choisies depuis **Configuration → Apparence du launcher**. Le
service de réglages pose `data-fond` sur `<html>` ; `web/src/styles.css` fait
le reste.

## Ce qui est ici aujourd'hui est PROVISOIRE

Les trois `.webp` sont des dégradés générés à la main, pour que l'interface ne
soit pas cassée en attendant de vraies captures. Ils font leur travail — le
voile et le verre dépoli se règlent dessus — mais ce ne sont pas des images du
réseau.

**À remplacer** par des captures du serveur : le spawn, un paysage du Nether,
un de l'End. Même noms, même format.

## Les contraintes, et pourquoi elles existent

| Contrainte              | Raison                                                                                                                                                                                                                                                                               |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1920 × 1080 au moins    | En dessous, `background-size: cover` étire sur un écran ordinaire, et le flou du verre fait ressortir les artefacts.                                                                                                                                                                 |
| WebP, qualité ~80       | Le launcher embarque ces fichiers : un PNG de 1920×1080 pèse plusieurs mégaoctets, contre quelques dizaines de kilooctets ici.                                                                                                                                                       |
| Plutôt sombres          | Le voile est **borné par le bas** à 44 % — voir `mc-reglages` : c'est ce qu'il faut pour tenir le contraste sur du blanc pur. Une image claire ne casserait donc rien, mais elle consommerait toute la marge du curseur et ne laisserait au joueur qu'un réglage sans effet visible. |
| Peu de détail au centre | C'est là que passent les cartes et le bouton. Un détail chargé sous du texte le rend illisible, quel que soit le voile.                                                                                                                                                              |

## Après remplacement : rien à remesurer

Le plancher du voile (`VOILE_PLANCHER` dans `crates/mc-reglages/src/bornes.rs`)
est **une mesure, pas un choix** — mais elle ne porte PAS sur ces images.

La première rédaction disait le contraire, et elle était fausse deux fois. Sur
les dégradés d'aujourd'hui, le texte tient déjà 10,9:1 **sans aucun voile** :
la mesure « sur l'image embarquée la plus claire » rend donc zéro, et n'aurait
garanti rien du tout. Pire, elle aurait adossé une garantie d'accessibilité à
des fichiers dont ce même document annonce le remplacement.

Le plancher se mesure sur le **pire fond concevable** — une surface blanche —
à travers le verre le plus léger de l'interface. Il ne dépend donc d'aucune
image, et remplacer celles-ci n'oblige à rien.

Ce qui reste vrai : une image très claire mangerait toute la marge du curseur.
Le launcher resterait lisible, mais « assombrissement » n'assombrirait plus
rien de perceptible. D'où la contrainte « plutôt sombres » du tableau
ci-dessus, qui est une exigence de direction artistique et non
d'accessibilité.
