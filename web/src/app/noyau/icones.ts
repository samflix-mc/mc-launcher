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
 *
 * ## La liste vient du design system
 *
 * `assets/Icons` du design system en fixe quarante-quatre ; celles qui suivent
 * sont celles que les gabarits emploient réellement. En ajouter une qui n'est
 * pas dans le design system demande d'abord de vérifier qu'aucune de celles-là
 * ne dit la même chose.
 */
export {
  // La pilule de navigation — les trois sections du design system.
  Compass,
  Newspaper,
  SlidersHorizontal,
  // La barre de titre et les fenêtres.
  Minus,
  Square,
  Copy,
  X,
  Bell,
  ChevronDown,
  // Le bouton de jeu, et ce qui l'entoure.
  Play,
  Download,
  RefreshCw,
  Rocket,
  // Les états.
  Check,
  CircleCheck,
  CircleX,
  Info,
  TriangleAlert,
  WifiOff,
  Wrench,
  Clock,
  Globe,
  Users,
  Package,
  Server,
  ShieldCheck,
  // La configuration.
  FolderOpen,
  Terminal,
  MemoryStick,
  Monitor,
  Trash2,
  // Les nouvelles et le compte.
  ArrowUpRight,
  ChevronRight,
  ExternalLink,
  Pin,
  LogOut,
  User,
} from 'lucide-angular';
