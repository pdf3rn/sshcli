import {
  Activity,
  ArrowDownToLine,
  ArrowRight,
  ArrowUpFromLine,
  ArrowUpDown,
  Columns2,
  EllipsisVertical,
  File,
  Folder,
  Pencil,
  Plus,
  Route,
  Star,
  Terminal,
  Trash2,
  X,
  type LucideProps,
} from 'lucide-react';

type IconProps = {
  size?: number;
  filled?: boolean;
};

const iconProps = (size: number): LucideProps => ({
  size,
  strokeWidth: 1.8,
  'aria-hidden': true,
});

export function PlusIcon({ size = 16 }: IconProps) {
  return <Plus {...iconProps(size)} />;
}

export function ColumnsIcon({ size = 16 }: IconProps) {
  return <Columns2 {...iconProps(size)} />;
}

export function XIcon({ size = 16 }: IconProps) {
  return <X {...iconProps(size)} />;
}

export function ActivityIcon({ size = 16 }: IconProps) {
  return <Activity {...iconProps(size)} />;
}

export function FolderIcon({ size = 14 }: IconProps) {
  return <Folder {...iconProps(size)} />;
}

export function FileIcon({ size = 14 }: IconProps) {
  return <File {...iconProps(size)} />;
}

export function StarIcon({ size = 14, filled = false }: IconProps) {
  return <Star {...iconProps(size)} fill={filled ? 'currentColor' : 'none'} />;
}

export function SftpIcon({ size = 14 }: IconProps) {
  return <ArrowUpDown {...iconProps(size)} />;
}

export function TunnelsIcon({ size = 14 }: IconProps) {
  return <Route {...iconProps(size)} />;
}

export function ConnectIcon({ size = 14 }: IconProps) {
  return <ArrowRight {...iconProps(size)} />;
}

export function EditIcon({ size = 14 }: IconProps) {
  return <Pencil {...iconProps(size)} />;
}

export function TrashIcon({ size = 14 }: IconProps) {
  return <Trash2 {...iconProps(size)} />;
}

export function ImportIcon({ size = 16 }: IconProps) {
  return <ArrowUpFromLine {...iconProps(size)} />;
}

export function ExportIcon({ size = 16 }: IconProps) {
  return <ArrowDownToLine {...iconProps(size)} />;
}

export function TerminalIcon({ size = 16 }: IconProps) {
  return <Terminal {...iconProps(size)} />;
}

export function MoreVerticalIcon({ size = 16 }: IconProps) {
  return <EllipsisVertical {...iconProps(size)} />;
}
