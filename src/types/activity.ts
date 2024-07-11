export type Activity = {
  name: string;
  type: number;
  url?: string;
  created_at: number;
  timestamps: TimeStamps;
  application_id?: string; // snowflake
  details?: string;
  state?: string;
  emoji?: Emoji;
  party?: Party;
  assets?: Assets;
  secrets?: Secrets;
  instance?: boolean;
  flag?: number;
  buttons?: Button[];
};

type TimeStamps = {
  start?: number;
  end?: number;
};

type Emoji = {
  name: string;
  id?: string; // snowflake
  animated?: boolean;
};

type Party = {
  id?: string;
  size?: [number, number];
};

type Assets = {
  large_image?: string;
  large_text?: string;
  small_image?: string;
  small_text?: string;
};

type Secrets = {
  join?: string;
  spectate?: string;
  match?: string;
};

type Button = {
  label: string;
  url: string;
};
