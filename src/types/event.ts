export type VoiceState = {
  nick: string;
  voice_state: {
    mute: boolean;
    deaf: boolean;
    self_mute: boolean;
    self_deaf: boolean;
  };
  user: {
    id: string;
    username: string;
    avatar: string;
  };
};

export type VCSelectPayload = {
  in_vc: boolean;
};

export type VCInfoPayload = {
  name: string;
  users: VoiceState[];
};

export type VCMuteUpdatePayload = {
  mute: boolean;
  deaf: boolean;
};

// LEAVE event does not have data field but theyre not used in event processing
export type VCUserPayload = {
  event: 'JOIN' | 'UPDATE' | 'LEAVE';
  data: {
    id: string;
    username: string;
    avatar: string;
    nick: string;
    mute: boolean;
    self_mute: boolean;
    deaf: boolean;
    self_deaf: boolean;
  };
};

export type VCSpeakPayload = {
  user_id: string;
  is_me: boolean;
  speaking: boolean;
};
