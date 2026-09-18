/**
 * The launcher's icons, named here and nowhere else.
 *
 * ## Why a file and not scattered imports
 *
 * Lucide publishes more than fifteen hundred of them. Importing them from
 * each component would give a list no one could read in one pass — and no
 * one arriving would know which icons the launcher already uses. One single
 * list, and you can see at a glance whether a fitting one already exists.
 *
 * ## Why we pass `[img]` rather than `LucideAngularModule.pick()`
 *
 * `pick()` feeds a global registry, resolved by NAME at runtime: an icon
 * forgotten from the registry renders blank, with no compile error. Passing
 * the icon node directly makes a typo a TypeScript error — and tree-shaking
 * only keeps what's referenced here.
 *
 * ## The list comes from the design system
 *
 * The design system's `assets/Icons` sets forty-four; the ones below are
 * the ones the templates actually use. Adding one that isn't in the design
 * system first requires checking that none of those already says the same
 * thing.
 */
export {
  // The navigation pill — the design system's three sections.
  Compass,
  Newspaper,
  SlidersHorizontal,
  // The title bar and windows.
  Minus,
  Square,
  Copy,
  X,
  Bell,
  ChevronDown,
  // The play button, and what surrounds it.
  Play,
  Download,
  RefreshCw,
  Rocket,
  // The states.
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
  // Settings.
  FolderOpen,
  Terminal,
  MemoryStick,
  Monitor,
  Trash2,
  // News and the account.
  ArrowUpRight,
  ChevronRight,
  ExternalLink,
  Pin,
  LogOut,
  User,
} from 'lucide-angular';
