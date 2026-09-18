/**
 * Les icônes du launcher, nommées ici et nulle part ailleurs.
 *
 * ## Pourquoi un fichier et non des imports éparpillés
 *
 * Lucide en publie plus de mille cinq cents. Les importer depuis chaque
 * composant donnerait une liste qu'on ne peut pas lire d'un trait — et
 * personne ne saurait, en arrivant, quelles icônes le launcher emploie déjà.
 * Une seule liste, et l'on voit tout de suite s'il en existe déjà une qui
 * convient.
 *
 * ## Pourquoi on passe `[img]` plutôt que `LucideAngularModule.pick()`
 *
 * `pick()` alimente un registre global, résolu par NOM à l'exécution : une
 * icône oubliée du registre rend un vide, sans erreur de compilation. En
 * passant le nœud d'icône directement, une faute de frappe est une erreur
 * TypeScript — et le tree-shaking ne garde que ce qui est référencé ici.
 */
export {
  // La navigation.
  House,
  Newspaper,
  Settings,
  Settings2,
  LogOut,
  // La barre de titre.
  Minus,
  Square,
  Copy,
  X,
  // Les actions.
  Play,
  Download,
  RefreshCw,
  FolderOpen,
  ShieldCheck,
  AlertTriangle,
  ExternalLink,
} from 'lucide-angular';
