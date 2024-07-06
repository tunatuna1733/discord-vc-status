import { VoiceState } from '../types/event';
import { UserData } from '../types/user';

export const isUserSpeaking = (userId: string, userList: UserData[]) => {
  const userData = userList.find((u) => u.id === userId);
  if (!userData) return false;
  return userData.speaking;
};

export const formatUserData = (data: VoiceState): UserData => {
  const mute =
    data.voice_states.mute || data.voice_states.self_mute || data.voice_states.deaf || data.voice_states.self_deaf;
  const deaf = data.voice_states.deaf || data.voice_states.self_deaf;
  const userData: UserData = {
    id: data.user.id,
    username: data.user.username,
    avatar: data.user.avatar,
    nick: data.nick,
    mute,
    deaf,
    speaking: false,
  };
  return userData;
};
