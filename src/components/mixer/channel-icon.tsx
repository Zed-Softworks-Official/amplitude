import {
    GamepadIcon,
    GlobeIcon,
    HeadphonesIcon,
    Mic2Icon,
    MonitorIcon,
} from 'lucide-react'
import type { ChannelId } from './types'

const iconMap: Record<ChannelId, React.ElementType> = {
    mic: Mic2Icon,
    system: MonitorIcon,
    browser: GlobeIcon,
    vc: HeadphonesIcon,
    game: GamepadIcon,
}

export function ChannelIcon({
    type,
    className,
}: {
    type: ChannelId
    className?: string
}) {
    const Icon = iconMap[type]
    return <Icon className={className} />
}
