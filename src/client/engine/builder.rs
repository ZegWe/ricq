use std::collections::HashMap;
use std::sync::atomic::Ordering;

use bytes::{BufMut, Bytes, BytesMut};
use chrono::Utc;
use jcers::JcePut;
use prost::Message;

use crate::client::outcome::PbToBytes;
use crate::client::protocol::{
    oicq::{self},
    packet::{EncryptType, Packet, PacketType},
};
use crate::jce::*;
use crate::pb;
use crate::pb::msg::SyncCookie;

fn pack_uni_request_data(data: &[u8]) -> Bytes {
    let mut r = BytesMut::new();
    r.put_slice(&[0x0A]);
    r.put_slice(data);
    r.put_slice(&[0x0B]);
    Bytes::from(r)
}

impl super::Engine {
    pub fn build_oicq_request_packet(&self, uin: i64, command_id: u16, body: &[u8]) -> Bytes {
        let req = oicq::Message {
            uin: uin as u32,
            command: command_id,
            body: Bytes::from(body.to_vec()),
            encryption_method: oicq::EncryptionMethod::ECDH,
        };
        self.oicq_codec.encode(req)
    }

    pub fn uni_packet_with_seq(&self, seq: u16, command: &str, body: Bytes) -> Packet {
        Packet {
            packet_type: PacketType::Simple,
            encrypt_type: EncryptType::D2Key,
            seq_id: seq as i32,
            body,
            command_name: command.to_owned(),
            uin: self.uin.load(Ordering::Relaxed),
            ..Default::default()
        }
    }

    pub fn uni_packet(&self, command: &str, body: Bytes) -> Packet {
        let seq = self.next_seq();
        self.uni_packet_with_seq(seq, command, body)
    }
}

impl super::Engine {
    pub fn build_client_register_packet(&self) -> Packet {
        let seq = self.next_seq();
        let transport = &self.transport;

        let svc = SvcReqRegister {
            uin: self.uin.load(Ordering::SeqCst),
            bid: 1 | 2 | 4,
            conn_type: 0,
            status: 11,
            kick_pc: 0,
            kick_weak: 0,
            ios_version: transport.device.version.sdk as i64,
            net_type: 1,
            reg_type: 0,
            guid: transport.sig.guid.to_owned(),
            is_set_status: 0,
            locale_id: 2052,
            dev_name: transport.device.model.to_owned(),
            dev_type: transport.device.model.to_owned(),
            os_ver: transport.device.version.release.to_owned(),
            open_push: 1,
            large_seq: 1551,
            old_sso_ip: 0,
            new_sso_ip: 31806887127679168,
            channel_no: "".to_string(),
            cpid: 0,
            vendor_name: transport.device.vendor_name.to_owned(),
            vendor_os_name: transport.device.vendor_os_name.to_owned(),
            b769: Bytes::from_static(&[
                0x0A, 0x04, 0x08, 0x2E, 0x10, 0x00, 0x0A, 0x05, 0x08, 0x9B, 0x02, 0x10, 0x00,
            ]),
            set_mute: 0,
            ..Default::default()
        };
        let mut b = BytesMut::new();
        b.put_slice(&[0x0A]);
        b.put_slice(&svc.freeze());
        b.put_slice(&[0x0B]);
        let buf = RequestDataVersion3 {
            map: HashMap::from([("SvcReqRegister".to_string(), b.into())]),
        };
        let pkt = RequestPacket {
            i_version: 3,
            s_servant_name: "PushService".to_string(),
            s_func_name: "SvcReqRegister".to_string(),
            s_buffer: buf.freeze(),
            context: Default::default(),
            status: Default::default(),
            ..Default::default()
        };
        Packet {
            packet_type: PacketType::Login,
            encrypt_type: EncryptType::D2Key,
            seq_id: seq as i32,
            body: pkt.freeze(),
            command_name: "StatSvc.register".into(),
            uin: self.uin.load(Ordering::SeqCst),
            ..Default::default()
        }
    }

    pub fn build_friend_group_list_request_packet(
        &self,
        friend_start_index: i16,
        friend_list_count: i16,
        group_start_index: i16,
        group_list_count: i16,
    ) -> Packet {
        let mut d50 = BytesMut::new();
        prost::Message::encode(
            &pb::D50ReqBody {
                appid: 1002,
                req_music_switch: 1,
                req_mutualmark_alienation: 1,
                req_ksing_switch: 1,
                req_mutualmark_lbsshare: 1,
                ..Default::default()
            },
            &mut d50,
        )
        .unwrap();

        let req = FriendListRequest {
            reqtype: 3,
            if_reflush: if friend_start_index <= 0 { 0 } else { 1 },
            uin: self.uin.load(Ordering::SeqCst),
            start_index: friend_start_index,
            friend_count: friend_list_count,
            group_id: 0,
            if_get_group_info: if group_list_count <= 0 { 0 } else { 1 },
            group_start_index: group_start_index as u8,
            group_count: group_list_count as u8,
            if_get_msf_group: 0,
            if_show_term_type: 1,
            version: 27,
            uin_list: vec![],
            app_type: 0,
            if_get_dov_id: 0,
            if_get_both_flag: 0,
            d50: Bytes::from(d50),
            d6b: Bytes::new(),
            sns_type_list: vec![13580, 13581, 13582],
        };
        let buf = RequestDataVersion3 {
            map: HashMap::from([("FL".to_string(), pack_uni_request_data(&req.freeze()))]),
        };
        let pkt = RequestPacket {
            i_version: 3,
            c_packet_type: 0x003,
            i_request_id: 1921334514,
            s_servant_name: "mqq.IMService.FriendListServiceServantObj".to_string(),
            s_func_name: "GetFriendListReq".to_string(),
            s_buffer: buf.freeze(),
            context: Default::default(),
            status: Default::default(),
            ..Default::default()
        };
        self.uni_packet("friendlist.getFriendGroupList", pkt.freeze())
    }

    pub fn build_system_msg_new_group_packet(&self, suspicious: bool) -> Packet {
        let req = pb::structmsg::ReqSystemMsgNew {
            msg_num: 100,
            version: 1000,
            checktype: 3,
            flag: Some(pb::structmsg::FlagInfo {
                grp_msg_kick_admin: 1,
                grp_msg_hidden_grp: 1,
                grp_msg_wording_down: 1,
                grp_msg_get_official_account: 1,
                grp_msg_get_pay_in_group: 1,
                frd_msg_discuss2_many_chat: 1,
                grp_msg_not_allow_join_grp_invite_not_frd: 1,
                frd_msg_need_waiting_msg: 1,
                frd_msg_uint32_need_all_unread_msg: 1,
                grp_msg_need_auto_admin_wording: 1,
                grp_msg_get_transfer_group_msg_flag: 1,
                grp_msg_get_quit_pay_group_msg_flag: 1,
                grp_msg_support_invite_auto_join: 1,
                grp_msg_mask_invite_auto_join: 1,
                grp_msg_get_disbanded_by_admin: 1,
                grp_msg_get_c2c_invite_join_group: 1,
                ..Default::default()
            }),
            friend_msg_type_flag: 1,
            req_msg_type: if suspicious { 2 } else { 1 },
            ..Default::default()
        };
        let payload = req.to_bytes();
        self.uni_packet("ProfileService.Pb.ReqSystemMsgNew.Group", payload)
    }

    pub fn build_group_list_request_packet(&self, vec_cookie: &[u8]) -> Packet {
        let req = TroopListRequest {
            uin: self.uin.load(Ordering::SeqCst),
            get_msf_msg_flag: 1,
            cookies: Bytes::from(vec_cookie.to_vec()),
            group_info: vec![],
            group_flag_ext: 1,
            version: 7,
            company_id: 0,
            version_num: 1,
            get_long_group_name: 1,
        };
        let buf = RequestDataVersion3 {
            map: HashMap::from([(
                "GetTroopListReqV2Simplify".to_string(),
                pack_uni_request_data(&req.freeze()),
            )]),
        };
        let pkt = RequestPacket {
            i_version: 3,
            c_packet_type: 0x00,
            i_message_type: 0,
            i_request_id: self.next_packet_seq(),
            s_servant_name: "mqq.IMService.FriendListServiceServantObj".to_string(),
            s_func_name: "GetTroopListReqV2Simplify".to_string(),
            s_buffer: buf.freeze(),
            context: Default::default(),
            status: Default::default(),
            ..Default::default()
        };
        self.uni_packet("friendlist.GetTroopListReqV2", pkt.freeze())
    }

    pub fn build_group_sending_packet(
        &self,
        group_code: i64,
        r: i32,
        pkg_num: i32,
        pkg_index: i32,
        pkg_div: i32,
        forward: bool,
        elems: Vec<pb::msg::Elem>,
    ) -> Packet {
        let req = pb::msg::SendMessageRequest {
            routing_head: Some(pb::msg::RoutingHead {
                c2c: None,
                grp: Some(pb::msg::Grp {
                    group_code: Some(group_code),
                }),
                grp_tmp: None,
                wpa_tmp: None,
            }),
            content_head: Some(pb::msg::ContentHead {
                pkg_num: Some(pkg_num),
                pkg_index: Some(pkg_index),
                div_seq: Some(pkg_div),
                auto_reply: None,
            }),
            msg_body: Some(pb::msg::MessageBody {
                rich_text: Some(pb::msg::RichText {
                    elems,
                    attr: None,
                    not_online_file: None,
                    ptt: None,
                }),
                msg_content: None,
                msg_encrypt_content: None,
            }),
            msg_seq: Some(self.next_group_seq()),
            msg_rand: Some(r),
            sync_cookie: Some(Vec::new()),
            msg_via: Some(1),
            msg_ctrl: if forward {
                Some(pb::msg::MsgCtrl { msg_flag: Some(4) })
            } else {
                None
            },
            data_statist: None,
            multi_send_seq: None,
        };
        self.uni_packet("MessageSvc.PbSendMsg", req.to_bytes())
    }

    pub fn build_group_member_info_request_packet(&self, group_code: i64, uin: i64) -> Packet {
        let payload = pb::GroupMemberReqBody {
            group_code,
            uin,
            new_client: true,
            client_type: 1,
            rich_card_name_ver: 1,
        };
        self.uni_packet(
            "group_member_card.get_group_member_card_info",
            payload.to_bytes(),
        )
    }

    pub fn build_group_member_list_request_packet(
        &self,
        group_uin: i64,
        group_code: i64,
        next_uin: i64,
    ) -> Packet {
        let payload = TroopMemberListRequest {
            uin: self.uin.load(Ordering::SeqCst),
            group_code,
            next_uin,
            group_uin,
            version: 2,
            ..Default::default()
        };
        let mut b = BytesMut::new();
        b.put_slice(&[0x0A]);
        b.put_slice(&payload.freeze());
        b.put_slice(&[0x0B]);
        let buf = RequestDataVersion3 {
            map: HashMap::from([("GTML".to_string(), b.into())]),
        };
        let pkt = RequestPacket {
            i_version: 3,
            i_request_id: self.next_packet_seq(),
            s_servant_name: "mqq.IMService.FriendListServiceServantObj".to_string(),
            s_func_name: "GetTroopMemberListReq".to_string(),
            s_buffer: buf.freeze(),
            ..Default::default()
        };
        self.uni_packet("friendlist.GetTroopMemberListReq", pkt.freeze())
    }

    pub fn build_delete_online_push_packet(
        &self,
        uin: i64,
        svrip: i32,
        push_token: Bytes,
        seq: u16,
        del_msg: Vec<PushMessageInfo>,
    ) -> Packet {
        let mut req = SvcRespPushMsg {
            uin,
            svrip,
            push_token,
            ..Default::default()
        };
        for m in del_msg {
            req.del_infos.push(DelMsgInfo {
                from_uin: m.from_uin,
                msg_time: m.msg_time,
                msg_seq: m.msg_seq,
                msg_cookies: m.msg_cookies,
                ..Default::default()
            })
        }
        let b = pack_uni_request_data(&req.freeze());
        let buf = RequestDataVersion3 {
            map: HashMap::from([("resp".to_string(), b.into())]),
        };
        let pkt = RequestPacket {
            i_version: 3,
            i_request_id: seq as i32,
            s_servant_name: "OnlinePush".to_string(),
            s_func_name: "SvcRespPushMsg".to_string(),
            s_buffer: buf.freeze(),
            ..Default::default()
        };
        self.uni_packet("OnlinePush.RespPush", pkt.freeze())
    }

    pub fn build_conf_push_resp_packet(&self, t: i32, pkt_seq: i64, jce_buf: Bytes) -> Packet {
        let mut req = jcers::JceMut::new();
        req.put_i32(t, 1);
        req.put_i64(pkt_seq, 2);
        req.put_bytes(jce_buf, 3);

        let buf = RequestDataVersion3 {
            map: HashMap::from([("PushResp".to_string(), pack_uni_request_data(&req.freeze()))]),
        };
        let pkt = RequestPacket {
            i_version: 3,
            s_servant_name: "QQService.ConfigPushSvc.MainServant".to_string(),
            s_func_name: "PushResp".to_string(),
            s_buffer: buf.freeze(),
            context: Default::default(),
            status: Default::default(),
            ..Default::default()
        };
        self.uni_packet("ConfigPushSvc.PushResp", pkt.freeze())
    }

    pub fn build_get_offline_msg_request_packet(&self, last_message_time: i64) -> Packet {
        let transport = &self.transport;
        let reg_req = SvcReqRegisterNew {
            request_optional: 0x101C2 | 32,
            c2c_msg: SvcReqGetMsgV2 {
                uin: self.uin.load(Ordering::SeqCst),
                date_time: match last_message_time {
                    0 => 1,
                    _ => last_message_time as i32,
                },
                recive_pic: 1,
                ability: 15,
                channel: 4,
                inst: 1,
                channel_ex: 1,
                sync_cookie: transport.sig.sync_cookie.to_owned(),
                sync_flag: 0,
                ramble_flag: 0,
                general_abi: 1,
                pub_account_cookie: transport.sig.pub_account_cookie.to_owned(),
            },
            group_msg: SvcReqPullGroupMsgSeq {
                verify_type: 0,
                filter: 1,
                ..Default::default()
            },
            end_seq: Utc::now().timestamp(),
            ..Default::default()
        };
        let flag = 0; // flag := msg.SyncFlag_START
        let msg_req = pb::msg::GetMessageRequest {
            sync_flag: Some(flag),
            sync_cookie: Some(transport.sig.sync_cookie.to_vec()),
            ramble_flag: Some(0),
            context_flag: Some(1),
            online_sync_flag: Some(0),
            latest_ramble_number: Some(20),
            other_ramble_number: Some(3),
            ..Default::default()
        }
        .to_bytes();
        let mut buf = BytesMut::new();
        buf.put_slice(&[0, 0, 0, 0]);
        buf.put_slice(&msg_req);
        let buf = buf.freeze();
        let mut req = jcers::JceMut::new();
        req.put_bytes(buf, 0);
        let buf = RequestDataVersion3 {
            map: HashMap::from([
                ("req_PbOffMsg".to_string(), req.freeze()),
                (
                    "req_OffMsg".to_string(),
                    pack_uni_request_data(&reg_req.freeze()),
                ),
            ]),
        };
        let pkt = RequestPacket {
            i_version: 3,
            s_servant_name: "RegPrxySvc".to_string(),
            s_buffer: buf.freeze(),
            ..Default::default()
        };
        self.uni_packet("RegPrxySvc.getOffMsg", pkt.freeze())
    }

    pub fn build_sync_msg_request_packet(&self, last_message_time: i64) -> Packet {
        let transport = &self.transport;
        let oidb_req = pb::oidb::D769RspBody {
            config_list: vec![
                pb::oidb::D769ConfigSeq {
                    r#type: Some(46),
                    version: Some(0),
                },
                pb::oidb::D769ConfigSeq {
                    r#type: Some(283),
                    version: Some(0),
                },
            ],
            ..Default::default()
        }
        .to_bytes();
        let reg_req = SvcReqRegisterNew {
            request_optional: 128 | 64 | 256 | 2 | 8192 | 16384 | 65536,
            dis_group_msg_filter: 1,
            c2c_msg: SvcReqGetMsgV2 {
                uin: self.uin.load(Ordering::SeqCst),
                date_time: match last_message_time {
                    0 => 1,
                    _ => last_message_time as i32,
                },
                recive_pic: 1,
                ability: 15,
                channel: 4,
                inst: 1,
                channel_ex: 1,
                sync_cookie: transport.sig.sync_cookie.to_owned(),
                sync_flag: 0, // START
                ramble_flag: 0,
                general_abi: 1,
                pub_account_cookie: transport.sig.pub_account_cookie.to_owned(),
            },
            group_mask: 2,
            end_seq: rand::random::<u32>() as i64,
            _0769_body: oidb_req,
            ..Default::default()
        };
        let flag = 0; // flag := msg.SyncFlag_START
        let mut msg_req = pb::msg::GetMessageRequest {
            sync_flag: Some(flag),
            sync_cookie: Some(transport.sig.sync_cookie.to_vec()),
            ramble_flag: Some(0),
            context_flag: Some(1),
            online_sync_flag: Some(0),
            latest_ramble_number: Some(20),
            other_ramble_number: Some(3),
            msg_req_type: Some(1),
            ..Default::default()
        };
        let off_msg = msg_req.to_bytes();
        msg_req.msg_req_type = Some(2);
        msg_req.sync_cookie = None;
        msg_req.pubaccount_cookie = Some(transport.sig.pub_account_cookie.to_vec());
        let pub_msg = msg_req.to_bytes();
        let buf = RequestDataVersion3 {
            map: HashMap::from([
                ("req_PbOffMsg".to_string(), {
                    let mut w = jcers::JceMut::new();
                    w.put_bytes(
                        {
                            let mut b = BytesMut::new();
                            b.put_slice(&[0; 4]);
                            b.put_slice(&off_msg);
                            b.freeze()
                        },
                        0,
                    );
                    w.freeze()
                }),
                ("req_PbPubMsg".to_string(), {
                    let mut w = jcers::JceMut::new();
                    w.put_bytes(
                        {
                            let mut b = BytesMut::new();
                            b.put_slice(&[0; 4]);
                            b.put_slice(&pub_msg);
                            b.freeze()
                        },
                        0,
                    );
                    w.freeze()
                }),
                (
                    "req_OffMsg".to_string(),
                    pack_uni_request_data(&reg_req.freeze()),
                ),
            ]),
        };
        let pkt = RequestPacket {
            i_version: 3,
            s_servant_name: "RegPrxySvc".to_string(),
            s_buffer: buf.freeze(),
            ..Default::default()
        };
        self.uni_packet("RegPrxySvc.infoSync", pkt.freeze())
    }

    pub fn build_group_msg_readed_packet(&self, group_code: i64, msg_seq: i32) -> Packet {
        let req = pb::msg::PbMsgReadedReportReq {
            grp_read_report: vec![pb::msg::PbGroupReadedReportReq {
                group_code: Some(group_code as u64),
                last_read_seq: Some(msg_seq as u64),
            }],
            ..Default::default()
        };
        self.uni_packet("PbMessageSvc.PbMsgReadedReport", req.to_bytes())
    }

    pub fn build_private_msg_readed_packet(&self, uin: i64, time: i64) -> Packet {
        let transport = &self.transport;
        let req = pb::msg::PbMsgReadedReportReq {
            c2_c_read_report: Some(pb::msg::PbC2cReadedReportReq {
                pair_info: vec![pb::msg::UinPairReadInfo {
                    peer_uin: Some(uin as u64),
                    last_read_time: Some(time as u32),
                    ..Default::default()
                }],
                sync_cookie: Some(transport.sig.sync_cookie.to_vec()),
                ..Default::default()
            }),
            ..Default::default()
        };
        self.uni_packet("PbMessageSvc.PbMsgReadedReport", req.to_bytes())
    }

    pub fn build_device_list_request_packet(&self) -> Packet {
        let transport = &self.transport;
        let req = SvcReqGetDevLoginInfo {
            guid: transport.sig.guid.to_owned(),
            login_type: 1,
            app_name: "com.tencent.mobileqq".into(),
            require_max: 20,
            get_dev_list_type: 20,
            ..Default::default()
        };
        let buf = RequestDataVersion3 {
            map: HashMap::from([(
                "SvcReqGetDevLoginInfo".to_string(),
                pack_uni_request_data(&req.freeze()),
            )]),
        };
        let pkt = RequestPacket {
            i_version: 3,
            s_servant_name: "StatSvc".to_string(),
            s_func_name: "SvcReqGetDevLoginInfo".to_string(),
            s_buffer: buf.freeze(),
            ..Default::default()
        };
        self.uni_packet("StatSvc.GetDevLoginInfo", pkt.freeze())
    }

    pub fn build_group_info_request_packet(&self, group_code: i64) -> Packet {
        let transport = &self.transport;
        let body = pb::oidb::D88dReqBody {
            app_id: Some(transport.version.app_id),
            req_group_info: vec![pb::oidb::ReqGroupInfo {
                group_code: Some(group_code as u64),
                stgroupinfo: Some(pb::oidb::D88dGroupInfo {
                    group_owner: Some(0),
                    group_uin: Some(0),
                    group_create_time: Some(0),
                    group_flag: Some(0),
                    group_member_max_num: Some(0),
                    group_member_num: Some(0),
                    group_option: Some(0),
                    group_level: Some(0),
                    group_face: Some(0),
                    group_name: Some(vec![]),
                    group_memo: Some(vec![]),
                    group_finger_memo: Some(vec![]),
                    group_last_msg_time: Some(0),
                    group_cur_msg_seq: Some(0),
                    group_question: Some(vec![]),
                    group_answer: Some(vec![]),
                    group_grade: Some(0),
                    active_member_num: Some(0),
                    head_portrait_seq: Some(0),
                    msg_head_portrait: Some(pb::oidb::D88dGroupHeadPortrait::default()),
                    st_group_ex_info: Some(pb::oidb::D88dGroupExInfoOnly::default()),
                    group_sec_level: Some(0),
                    cmduin_privilege: Some(0),
                    no_finger_open_flag: Some(0),
                    no_code_finger_open_flag: Some(0),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            pc_client_version: Some(0),
        };
        let payload = pb::oidb::OidbssoPkg {
            command: 2189,
            bodybuffer: body.to_bytes().to_vec(),
            ..Default::default()
        };
        self.uni_packet("OidbSvc.0x88d_0", payload.to_bytes())
    }

    pub fn build_get_message_request_packet(&self, flag: i32, time: i64) -> Packet {
        let mut cook = { self.transport.sig.sync_cookie.to_vec() };
        if cook.is_empty() {
            cook = SyncCookie {
                time: Some(time),
                time1: None,
                ran1: Some(758330138),
                ran2: Some(2480149246),
                const1: Some(1167238020),
                const2: Some(3913056418),
                const3: Some(0x1D),
                const4: None,
                last_sync_time: None,
            }
            .encode_to_vec();
        }
        let req = pb::msg::GetMessageRequest {
            sync_flag: Some(flag),
            sync_cookie: Some(cook),
            latest_ramble_number: Some(20),
            other_ramble_number: Some(3),
            online_sync_flag: Some(1),
            context_flag: Some(1),
            msg_req_type: Some(1),
            pubaccount_cookie: Some(vec![]),
            msg_ctrl_buf: Some(vec![]),
            server_buf: Some(vec![]),
            ..Default::default()
        };
        self.uni_packet("MessageSvc.PbGetMsg", req.to_bytes())
    }
}

// #[cfg(test)]
// mod tests {
//     use bytes::BufMut;
//     use chrono::Utc;
//     use rand::distributions::Alphanumeric;
//     use rand::{Rng, thread_rng};

//     #[test]
//     fn test_read() {}
// }