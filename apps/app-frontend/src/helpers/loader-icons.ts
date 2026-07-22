import anvilIcon from '@/assets/minecraft-icons/Anvil.png'
import cleanroomIcon from '@/assets/minecraft-icons/Cleanroom.png'
import commandBlockIcon from '@/assets/minecraft-icons/CommandBlock.png'
import eggIcon from '@/assets/minecraft-icons/Egg.png'
import fabricIcon from '@/assets/minecraft-icons/Fabric.png'
import labymodIcon from '@/assets/minecraft-icons/LabyMod.png'
import neoforgeIcon from '@/assets/minecraft-icons/NeoForge.png'
import optifabricIcon from '@/assets/minecraft-icons/OptiFabric.png'
import quiltIcon from '@/assets/minecraft-icons/Quilt.png'

const loaderIconMap: Record<string, string> = {
	vanilla: commandBlockIcon,
	forge: anvilIcon,
	neoforge: neoforgeIcon,
	fabric: fabricIcon,
	quilt: quiltIcon,
	optifine: optifabricIcon,
	liteloader: eggIcon,
	cleanroom: cleanroomIcon,
	labymod: labymodIcon,
}

export function getLoaderIcon(loader: string): string {
	return loaderIconMap[loader] ?? commandBlockIcon
}
