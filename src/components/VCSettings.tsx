import { invoke } from '@tauri-apps/api/tauri';
import { Grid, IconButton } from '@mui/material';
import SettingsPhoneIcon from '@mui/icons-material/SettingsPhone';
import MicIcon from '@mui/icons-material/Mic';
import MicOffIcon from '@mui/icons-material/MicOff';
import HeadsetIcon from '@mui/icons-material/Headset';
import HeadsetOffIcon from '@mui/icons-material/HeadsetOff';
import CallEndIcon from '@mui/icons-material/CallEnd';
import { IpcError } from '../utils/error';

type Props = {
  inVC: boolean;
  isMute: boolean;
  isDeafen: boolean;
  isSpeaking: boolean;
  vcName: string;
};

const VCSettings = ({ inVC, isMute, isDeafen, isSpeaking, vcName }: Props) => {
  const disconnectVC = async () => {
    invoke('disconnect_vc').catch((e: IpcError) => {
      // failed to send disconnect payload
      console.error(e);
    });
  };

  const toggleMute = async () => {
    invoke('toggle_mute').catch((e: IpcError) => {
      console.error(e);
    });
  };

  const toggleDeafen = async () => {
    invoke('toggle_deafen').catch((e: IpcError) => {
      console.error(e);
    });
  };

  return (
    <Grid container>
      <Grid item xs={1} display={'flex'} alignItems={'center'} justifyContent={'center'}>
        <IconButton aria-label="chatting">
          <SettingsPhoneIcon />
        </IconButton>
      </Grid>
      <Grid item xs={8}>
        {vcName}
      </Grid>
      <Grid item xs={1} display={'flex'} alignItems={'center'} justifyContent={'center'}>
        <IconButton
          aria-label="mute"
          onClick={(e) => {
            e.preventDefault();
            toggleMute();
          }}
        >
          {inVC ? (
            isMute ? (
              <MicOffIcon color="error" />
            ) : isSpeaking ? (
              <MicIcon color="success" />
            ) : (
              <MicIcon />
            )
          ) : isMute ? (
            <MicOffIcon color="disabled" />
          ) : (
            <MicIcon color="disabled" />
          )}
        </IconButton>
      </Grid>
      <Grid item xs={1} display={'flex'} alignItems={'center'} justifyContent={'center'}>
        <IconButton
          aria-label="deafen"
          onClick={(e) => {
            e.preventDefault();
            toggleDeafen();
          }}
        >
          {inVC ? (
            isDeafen ? (
              <HeadsetOffIcon color="error" />
            ) : (
              <HeadsetIcon />
            )
          ) : isDeafen ? (
            <HeadsetOffIcon color="disabled" />
          ) : (
            <HeadsetIcon color="disabled" />
          )}
        </IconButton>
      </Grid>
      <Grid item xs={1} display={'flex'} alignItems={'center'} justifyContent={'center'}>
        <IconButton
          aria-label="disconnect"
          onClick={(e) => {
            e.preventDefault();
            disconnectVC();
          }}
        >
          <CallEndIcon />
        </IconButton>
      </Grid>
    </Grid>
  );
};

export default VCSettings;
