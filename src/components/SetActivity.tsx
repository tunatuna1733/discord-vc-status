import { Box } from '@mui/material';
import { useState } from 'react';
import { Activity } from '../types/activity';
import { invoke } from '@tauri-apps/api';

const SetActivity = () => {
  const [activityData, setActivityData] = useState<Activity>();
  const [isSet, setIsSet] = useState(false);

  const sendActivity = (activity: Activity) => {
    invoke('set_activity', { activity })
      .then(() => {
        setIsSet(true);
      })
      .catch((err) => {
        console.error(err);
      });
  };

  const clearActivity = () => {
    invoke('clear_activity')
      .then(() => {
        setIsSet(false);
      })
      .catch((err) => {
        console.error(err);
      });
  };

  return <Box></Box>;
};

export default SetActivity;
