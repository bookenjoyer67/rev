import { getActiveServer } from '$lib/stores/server';
import { getActiveAuth } from '$lib/stores/auth';

export const ssr = false;

export async function load({ params, fetch }) {
    const server = getActiveServer();
    if (!server) return { profile: null, error: 'Not connected to a server', endorsements: null };

    try {
        const [profileRes, endorsementsRes] = await Promise.all([
            fetch(`${server}/api/users/${params.id}`),
            fetch(`${server}/api/users/${params.id}/endorsements`),
        ]);

        if (!profileRes.ok) return { profile: null, error: 'User not found', endorsements: null };
        const profile = await profileRes.json();

        let endorsements = null;
        if (endorsementsRes.ok) {
            endorsements = await endorsementsRes.json();
        }

        const myId = getActiveAuth()?.userId || null;
        const myEndorsement = endorsements?.endorsements?.find((e: any) => e.endorser_id === myId);

        return { profile, isOwnProfile: myId === profile.id, endorsements, hasEndorsed: !!myEndorsement };
    } catch {
        return { profile: null, error: 'Failed to load profile', endorsements: null };
    }
}
