import { Box, Button, FormControl, Grid, InputLabel, MenuItem, Select, Stack, TextField } from '@mui/material';
import { useState } from 'react';
import { Activity } from '../types/activity';
import { invoke } from '@tauri-apps/api';

const defaultActivity: Activity = {
  // name: '',
  type: 0,
  // created_at: Date.now(),
};

const activityTypes = ['Game', 'Streaming', 'Listening', 'Watching', 'Emoji', 'Competing'];

const SetActivity = () => {
  const [activityData, setActivityData] = useState<Activity>(defaultActivity);
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

  return (
    <Stack spacing={2}>
      {/*<TextField
        label="Name"
        variant="outlined"
        onChange={(e) => {
          setActivityData({ ...activityData, name: e.target.value });
        }}
      />*/}
      <FormControl>
        <InputLabel id="type-select">Type</InputLabel>
        <Select
          labelId="type-select"
          value={activityData.type}
          label="Type"
          onChange={(e) => {
            setActivityData({
              ...activityData,
              type: typeof e.target.value === 'string' ? parseInt(e.target.value) : e.target.value,
            });
          }}
        >
          {activityTypes.map((a, i) => (
            <MenuItem value={i} key={i}>
              {a}
            </MenuItem>
          ))}
        </Select>
      </FormControl>
      <TextField
        label="Detail"
        variant="outlined"
        onChange={(e) => {
          setActivityData({ ...activityData, details: e.target.value });
        }}
      />
      <TextField
        label="State"
        variant="outlined"
        onChange={(e) => {
          setActivityData({ ...activityData, state: e.target.value });
        }}
      />
      <Grid container>
        <Grid item xs={6}>
          <Button variant="outlined" color="info" disabled={!isSet} onClick={clearActivity}>
            Clear
          </Button>
        </Grid>
        <Grid item xs={6}>
          <Button
            variant="outlined"
            color="info"
            onClick={() => {
              sendActivity(activityData);
            }}
          >
            Set
          </Button>
        </Grid>
      </Grid>
    </Stack>
  );
};

export default SetActivity;
