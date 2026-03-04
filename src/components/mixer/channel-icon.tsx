import {
    GamepadIcon,
    GlobeIcon,
    HeadphonesIcon,
    Mic2Icon,
    MonitorIcon,
    Music2Icon,
    Volume2Icon,
} from 'lucide-react'

const NAME_TO_ICON: Record<string, React.ElementType> = {
    mic: Mic2Icon,
    system: MonitorIcon,
    browser: GlobeIcon,
    vc: HeadphonesIcon,
    game: GamepadIcon,
    music: Music2Icon,
}

export function ChannelIcon({
    name,
    className,
}: {
    name: string
    className?: string
}) {
    const Icon = NAME_TO_ICON[name.toLowerCase()] ?? Volume2Icon
    return <Icon className={className} />
}
