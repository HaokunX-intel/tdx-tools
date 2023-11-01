/*
 * Copyright (C) 2011-2021 Intel Corporation. All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 *   * Redistributions of source code must retain the above copyright
 *     notice, this list of conditions and the following disclaimer.
 *   * Redistributions in binary form must reproduce the above copyright
 *     notice, this list of conditions and the following disclaimer in
 *     the documentation and/or other materials provided with the
 *     distribution.
 *   * Neither the name of Intel Corporation nor the names of its
 *     contributors may be used to endorse or promote products derived
 *     from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
 * A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
 * OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
 * SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
 * LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 */

#include <stdio.h>
#include <vector>
#include <string>
#include <assert.h>
#include <fstream>
#include "sgx_quote_4.h"

#ifndef QVL_ONLY
#include <sgx_uae_launch.h>
#include "sgx_urts.h"
#include "Enclave_u.h"
#else
#include <cstring>
#endif
#include "sgx_ql_quote.h"
#include "sgx_dcap_quoteverify.h"
#ifdef SGX_QPL_LOGGING
#include "sgx_default_quote_provider.h"
#endif

#ifndef _MSC_VER

#define SAMPLE_ISV_ENCLAVE "enclave.signed.so"
#define DEFAULT_QUOTE "../QuoteGenerationSample/quote.dat"

#else

#define SAMPLE_ISV_ENCLAVE "enclave.signed.dll"
#define DEFAULT_QUOTE "..\\..\\..\\QuoteGenerationSample\\x64\\Debug\\quote.dat"

#define strncpy strncpy_s
#endif
#ifndef SGX_CDECL
#define SGX_CDECL
#endif

using namespace std;
#define log(msg, ...)                             \
    do                                            \
    {                                             \
        printf("[APP] " msg "\n", ##__VA_ARGS__); \
        fflush(stdout);                           \
    } while (0)

typedef union _supp_ver_t
{
    uint32_t version;
    struct
    {
        uint16_t major_version;
        uint16_t minor_version;
    };
} supp_ver_t;

vector<uint8_t> readBinaryContent(const string &filePath)
{
    ifstream file(filePath, ios::binary);
    if (!file.is_open())
    {
        log("Error: Unable to open quote file %s", filePath.c_str());
        return {};
    }

    file.seekg(0, ios_base::end);
    streampos fileSize = file.tellg();

    file.seekg(0, ios_base::beg);
    vector<uint8_t> retVal(fileSize);
    file.read(reinterpret_cast<char *>(retVal.data()), fileSize);
    file.close();
    return retVal;
}
#define PATHSIZE 0x418U

/**
 * @param quote - ECDSA quote buffer
 * @param use_qve - Set quote verification mode
 *                   If true, quote verification will be performed by Intel QvE
 *                   If false, quote verification will be performed by untrusted QVL
 */

int ecdsa_quote_verification(vector<uint8_t> quote, bool use_qve)
{
#ifndef QVL_ONLY
    sgx_status_t sgx_ret = SGX_SUCCESS;
    sgx_ql_qe_report_info_t qve_report_info;
    int updated = 0;
    sgx_launch_token_t token = {0};
    unsigned char rand_nonce[16] = "59jslk201fgjmm;";
    quote3_error_t verify_qveid_ret = SGX_QL_ERROR_UNEXPECTED;
    sgx_enclave_id_t eid = 0;
#else
    (void)use_qve;
#endif

    int ret = 0;
    time_t current_time = 0;
    quote3_error_t dcap_ret = SGX_QL_ERROR_UNEXPECTED;
    uint32_t collateral_expiration_status = 1;
    sgx_ql_qv_result_t quote_verification_result = SGX_QL_QV_RESULT_UNSPECIFIED;

    tee_supp_data_descriptor_t supp_data;

    // You can also set specify a major version in this structure, then we will always return supplemental data of the major version
    // set major verison to 0 means always return latest supplemental data
    memset(&supp_data, 0, sizeof(tee_supp_data_descriptor_t));

    supp_ver_t latest_ver;

 #ifndef QVL_ONLY
    // Trusted quote verification
    if (use_qve)
    {
        // set nonce
        //
        memcpy(qve_report_info.nonce.rand, rand_nonce, sizeof(rand_nonce));

        // get target info of SampleISVEnclave. QvE will target the generated report to this enclave.
        //
        sgx_ret = sgx_create_enclave(SAMPLE_ISV_ENCLAVE, SGX_DEBUG_FLAG, &token, &updated, &eid, NULL);
        if (sgx_ret != SGX_SUCCESS)
        {
            log("Error: Can't load SampleISVEnclave. 0x%04x", sgx_ret);
            return -1;
        }
        sgx_status_t get_target_info_ret;
        sgx_ret = ecall_get_target_info(eid, &get_target_info_ret, &qve_report_info.app_enclave_target_info);
        if (sgx_ret != SGX_SUCCESS || get_target_info_ret != SGX_SUCCESS)
        {
            log("Error in sgx_get_target_info. 0x%04x", get_target_info_ret);
            ret = -1;
            goto cleanup;
        }
        else
        {
            log("Info: get target info successfully returned.");
        }

        // call DCAP quote verify library to set QvE loading policy
        //
        dcap_ret = sgx_qv_set_enclave_load_policy(SGX_QL_DEFAULT);
        if (dcap_ret == SGX_QL_SUCCESS)
        {
            log("Info: sgx_qv_set_enclave_load_policy successfully returned.");
        }
        else
        {
            log("Error: sgx_qv_set_enclave_load_policy failed: 0x%04x", dcap_ret);
            ret = -1;
            goto cleanup;
        }

        // call DCAP quote verify library to get supplemental latest version and data size
        // version is a combination of major_version and minor version
        // you can set the major version in 'supp_data.major_version' to get old version supplemental data
        // only support major_version 3 right now
        dcap_ret = tee_get_supplemental_data_version_and_size(quote.data(),
                                                              (uint32_t)quote.size(),
                                                              &latest_ver.version,
                                                              &supp_data.data_size);

        if (dcap_ret == SGX_QL_SUCCESS && supp_data.data_size == sizeof(sgx_ql_qv_supplemental_t))
        {
            log("Info: tee_get_quote_supplemental_data_version_and_size successfully returned.");
            log("Info: latest supplemental data major version: %d, minor version: %d, size: %d", latest_ver.major_version, latest_ver.minor_version, supp_data.data_size);
            supp_data.p_data = (uint8_t *)malloc(supp_data.data_size);
            if (supp_data.p_data != NULL)
            {
                memset(supp_data.p_data, 0, supp_data.data_size);
            }

            // Just print error in sample
            //
            else
            {
                log("Error: Cannot allocate memory for supplemental data.");
                supp_data.data_size = 0;
            }
        }
        else
        {
            if (dcap_ret != SGX_QL_SUCCESS)
                log("Error: tee_get_quote_supplemental_data_size failed: 0x%04x", dcap_ret);

            if (supp_data.data_size != sizeof(sgx_ql_qv_supplemental_t))
                log("Warning: Quote supplemental data size is different between DCAP QVL and QvE, please make sure you installed DCAP QVL and QvE from same release.");

            supp_data.data_size = 0;
        }

        // set current time. This is only for sample use, please use trusted time in product.
        //
        current_time = time(NULL);

        // call DCAP quote verify library for quote verification
        // here you can choose 'trusted' or 'untrusted' quote verification by specifying parameter '&qve_report_info'
        // if '&qve_report_info' is NOT NULL, this API will call Intel QvE to verify quote
        // if '&qve_report_info' is NULL, this API will call 'untrusted quote verify lib' to verify quote, this mode doesn't rely on SGX capable system, but the results can not be cryptographically authenticated
        dcap_ret = tee_verify_quote(
            quote.data(), (uint32_t)quote.size(),
            NULL,
            current_time,
            &collateral_expiration_status,
            &quote_verification_result,
            &qve_report_info,
            &supp_data);
        if (dcap_ret == SGX_QL_SUCCESS)
        {
            log("Info: App: tee_verify_quote successfully returned.");
        }
        else
        {
            log("Error: App: tee_verify_quote failed: 0x%04x", dcap_ret);
            ret = -1;
            goto cleanup;
        }

        // Threshold of QvE ISV SVN. The ISV SVN of QvE used to verify quote must be greater or equal to this threshold
        // e.g. You can check latest QvE ISVSVN from QvE configuration file on Github
        // https://github.com/intel/SGXDataCenterAttestationPrimitives/blob/master/QuoteVerification/QvE/Enclave/linux/config.xml#L4
        // or you can get latest QvE ISVSVN in QvE Identity JSON file from
        // https://api.trustedservices.intel.com/sgx/certification/v4/qve/identity
        // Make sure you are using trusted & latest QvE ISV SVN as threshold
        // Warning: The function may return erroneous result if QvE ISV SVN has been modified maliciously.
        //
        sgx_isv_svn_t qve_isvsvn_threshold = 7;

        // call sgx_dcap_tvl API in SampleISVEnclave to verify QvE's report and identity
        //
        sgx_ret = sgx_tvl_verify_qve_report_and_identity(eid,
                                                         &verify_qveid_ret,
                                                         quote.data(),
                                                         (uint32_t)quote.size(),
                                                         &qve_report_info,
                                                         current_time,
                                                         collateral_expiration_status,
                                                         quote_verification_result,
                                                         supp_data.p_data,
                                                         supp_data.data_size,
                                                         qve_isvsvn_threshold);

        if (sgx_ret != SGX_SUCCESS || verify_qveid_ret != SGX_QL_SUCCESS)
        {
            log("Error: Ecall: Verify QvE report and identity failed. 0x%04x", verify_qveid_ret);
            ret = -1;
            goto cleanup;
        }
        else
        {
            log("Info: Ecall: Verify QvE report and identity successfully returned.");
        }

        // check verification result
        //
        switch (quote_verification_result)
        {
        case SGX_QL_QV_RESULT_OK:
            // check verification collateral expiration status
            // this value should be considered in your own attestation/verification policy
            //
            if (collateral_expiration_status == 0)
            {
                log("Info: App: Verification completed successfully.");
                ret = 0;
            }
            else
            {
                log("Warning: App: Verification completed, but collateral is out of date based on 'expiration_check_date' you provided.");
                ret = 1;
            }

            break;
        case SGX_QL_QV_RESULT_CONFIG_NEEDED:
        case SGX_QL_QV_RESULT_OUT_OF_DATE:
        case SGX_QL_QV_RESULT_OUT_OF_DATE_CONFIG_NEEDED:
        case SGX_QL_QV_RESULT_SW_HARDENING_NEEDED:
        case SGX_QL_QV_RESULT_CONFIG_AND_SW_HARDENING_NEEDED:
            log("Warning: App: Verification completed with Non-terminal result: %x", quote_verification_result);
            ret = 1;
            break;
        case SGX_QL_QV_RESULT_INVALID_SIGNATURE:
        case SGX_QL_QV_RESULT_REVOKED:
        case SGX_QL_QV_RESULT_UNSPECIFIED:
        default:
            log("Error: App: Verification completed with Terminal result: %x", quote_verification_result);
            ret = -1;
            break;
        }

        // check supplemental data if necessary
        //
        if (dcap_ret == SGX_QL_SUCCESS && supp_data.p_data != NULL && supp_data.data_size > 0)
        {
            sgx_ql_qv_supplemental_t *p = (sgx_ql_qv_supplemental_t *)supp_data.p_data;

            // you can check supplemental data based on your own attestation/verification policy
            // here we only print supplemental data version for demo usage
            //
            log("Info: Supplemental data Major Version: %d", p->major_version);
            log("Info: Supplemental data Minor Version: %d", p->minor_version);

            // print SA list if exist, SA list is supported from version 3.1
            //
            if (p->version > 3 && strlen(p->sa_list) > 0)
            {
                log("Info: Advisory ID: %s", p->sa_list);
            }
        }
    }
    // Untrusted quote verification
    else
#endif
    {
        // call DCAP quote verify library to get supplemental latest version and data size
        // version is a combination of major_version and minor version
        // you can set the major version in 'supp_data.major_version' to get old version supplemental data
        // only support major_version 3 right now
        dcap_ret = tee_get_supplemental_data_version_and_size(quote.data(),
                                                              (uint32_t)quote.size(),
                                                              &latest_ver.version,
                                                              &supp_data.data_size);

        if (dcap_ret == SGX_QL_SUCCESS && supp_data.data_size == sizeof(sgx_ql_qv_supplemental_t))
        {
            log("Info: tee_get_quote_supplemental_data_version_and_size successfully returned.");
            log("Info: latest supplemental data major version: %d, minor version: %d, size: %d", latest_ver.major_version, latest_ver.minor_version, supp_data.data_size);
            supp_data.p_data = (uint8_t *)malloc(supp_data.data_size);
            if (supp_data.p_data != NULL)
            {
                memset(supp_data.p_data, 0, supp_data.data_size);
            }

            // Just print error in sample
            //
            else
            {
                log("Error: Cannot allocate memory for supplemental data.");
                supp_data.data_size = 0;
            }
        }
        else
        {
            if (dcap_ret != SGX_QL_SUCCESS)
                log("Error: tee_get_quote_supplemental_data_size failed: 0x%04x", dcap_ret);

            if (supp_data.data_size != sizeof(sgx_ql_qv_supplemental_t))
                log("Warning: Quote supplemental data size is different between DCAP QVL and QvE, please make sure you installed DCAP QVL and QvE from same release.");

            supp_data.data_size = 0;
        }

        // set current time. This is only for sample purposes, in production mode a trusted time should be used.
        //
        current_time = time(NULL);

        // call DCAP quote verify library for quote verification
        // here you can choose 'trusted' or 'untrusted' quote verification by specifying parameter '&qve_report_info'
        // if '&qve_report_info' is NOT NULL, this API will call Intel QvE to verify quote
        // if '&qve_report_info' is NULL, this API will call 'untrusted quote verify lib' to verify quote, this mode doesn't rely on SGX capable system, but the results can not be cryptographically authenticated
        dcap_ret = tee_verify_quote(
            quote.data(), (uint32_t)quote.size(),
            NULL,
            current_time,
            &collateral_expiration_status,
            &quote_verification_result,
            NULL,
            &supp_data);
        if (dcap_ret == SGX_QL_SUCCESS)
        {
            log("Info: App: tee_verify_quote successfully returned.");
        }
        else
        {
            log("Error: App: tee_verify_quote failed: 0x%04x", dcap_ret);
            goto cleanup;
        }

        // check verification result
        //
        switch (quote_verification_result)
        {
        case SGX_QL_QV_RESULT_OK:
            // check verification collateral expiration status
            // this value should be considered in your own attestation/verification policy
            //
            if (collateral_expiration_status == 0)
            {
                log("Info: App: Verification completed successfully.");
                ret = 0;
            }
            else
            {
                log("Warning: App: Verification completed, but collateral is out of date based on 'expiration_check_date' you provided.");
                ret = 1;
            }
            break;
        case SGX_QL_QV_RESULT_CONFIG_NEEDED:
        case SGX_QL_QV_RESULT_OUT_OF_DATE:
        case SGX_QL_QV_RESULT_OUT_OF_DATE_CONFIG_NEEDED:
        case SGX_QL_QV_RESULT_SW_HARDENING_NEEDED:
        case SGX_QL_QV_RESULT_CONFIG_AND_SW_HARDENING_NEEDED:
            log("Warning: App: Verification completed with Non-terminal result: %x", quote_verification_result);
            ret = 1;
            break;
        case SGX_QL_QV_RESULT_INVALID_SIGNATURE:
        case SGX_QL_QV_RESULT_REVOKED:
        case SGX_QL_QV_RESULT_UNSPECIFIED:
        default:
            log("Error: App: Verification completed with Terminal result: %x", quote_verification_result);
            ret = -1;
            break;
        }

        // check supplemental data if necessary
        //
        if (dcap_ret == SGX_QL_SUCCESS && supp_data.p_data != NULL && supp_data.data_size > 0)
        {
            sgx_ql_qv_supplemental_t *p = (sgx_ql_qv_supplemental_t *)supp_data.p_data;

            // you can check supplemental data based on your own attestation/verification policy
            // here we only print supplemental data version for demo usage
            //
            log("Info: Supplemental data Major Version: %d", p->major_version);
            log("Info: Supplemental data Minor Version: %d", p->minor_version);

            // print SA list if exist, SA list is supported from version 3.1
            //
            if (p->version > 3 && strlen(p->sa_list) > 0)
            {
                log("Info: Advisory ID: %s", p->sa_list);
            }
        }
    }

cleanup:
    if (supp_data.p_data != NULL)
    {
        free(supp_data.p_data);
    }

#ifndef QVL_ONLY
    if (eid)
    {
        sgx_destroy_enclave(eid);
    }
#endif

    return ret;
}

void usage()
{
    log("Usage:");
    log("\tPlease specify quote path, e.g. \"./app -quote <path/to/quote>\"");
    log("\t\tDefault quote path is %s when no command line args", DEFAULT_QUOTE);
}

#ifdef SGX_QPL_LOGGING
void qpl_logger(sgx_ql_log_level_t level, const char *message)
{
    const string pre_qcnl = "[QCNL]";
    const string pre_qpl = "[QPL]";
    string msg(message);
    if (level == SGX_QL_LOG_INFO)
    {
        if (msg.find(pre_qcnl) == 0)
            msg.insert(pre_qcnl.length(), " Info: ");
        else if (msg.find(pre_qpl) == 0)
            msg.insert(pre_qcnl.length(), "Info: ");
        printf("%s", msg.c_str());
    }
    else if (level == SGX_QL_LOG_ERROR)
    {
        if (msg.find(pre_qcnl) == 0)
            msg.insert(pre_qcnl.length(), " Error: ");
        else if (msg.find(pre_qpl) == 0)
            msg.insert(pre_qcnl.length(), "Error: ");
        printf("%s", msg.c_str());
    }
}
#endif

#define BODY_SIZE 1024

/* Application entry */
int SGX_CDECL main(int argc, char *argv[])
{
    vector<uint8_t> quote;

    char quote_path[PATHSIZE] = {'\0'};

    uint8_t tdreport[BODY_SIZE] = {'\0'};
    int tdreport_size = 0;
    char *quote_char = NULL;

    // Just for sample use, better to change solid command line args solution in production env
    if (argc != 1 && argc != 3)
    {
        usage();
        return 0;
    }

    if (argv[1] && argv[2])
    {
        if (!strcmp(argv[1], "-quote"))
        {
            strncpy(quote_path, argv[2], PATHSIZE - 1);
        }
    }

    if (*quote_path == '\0')
    {
        strncpy(quote_path, DEFAULT_QUOTE, PATHSIZE - 1);
    }

    // read quote from file
    //
    quote = readBinaryContent(quote_path);
    if (quote.empty())
    {
        usage();
        return -1;
    }

    // printf("quote size: %ld\n", sizeof(sgx_quote4_t));
    quote_char = new char[quote.size()];
    std::copy(quote.begin(), quote.end(), quote_char);

    memset(tdreport, 0, BODY_SIZE);
    sgx_quote4_t* tmp_p_quote = (sgx_quote4_t*)quote_char;
    tdreport_size = sizeof(tmp_p_quote->report_body);
    memcpy(tdreport, (void*)(&(tmp_p_quote->report_body)), tdreport_size);

    printf("=============================================\n");
    printf("%d\n", tmp_p_quote->header.tee_type);
    printf("%d\n", tdreport_size);
    printf("%x %x %x %x %x %x %x %x\n", 
        tdreport[520], 
        tdreport[521],
        tdreport[522], 
        tdreport[523],
        tdreport[524], 
        tdreport[525],
        tdreport[526], 
        tdreport[527]);

    log("Info: ECDSA quote path: %s", quote_path);

    // When building with QVL_ONLY = 0 (default), two different types of quote verification are demonstrated
    //    a. Trusted quote verification - quote is verified with Intel QvE
    //    b. Untrusted quote verification - quote is verified with Intel QVL (Quote Verification Library)
    //       This mode does not rely on an SGX/TDX capable system, but the results cannot be cryptographically authenticated
    // If built with QVL_ONLY = 1, only one type of quote verification will be demonstrated
    //    a. Untrusted quote verification - quote is verified with Intel QVL (Quote Verification Library)
    //       This mode does not rely on an SGX/TDX capable system, but the results cannot be cryptographically authenticated

    // Trusted quote verification, ignore error checking
    log("Trusted quote verification:");
    ecdsa_quote_verification(quote, true);

    printf("\n===========================================\n\n");
    // Unrusted quote verification, ignore error checking
    log("Untrusted quote verification:");

    ecdsa_quote_verification(quote, false);
    

    delete quote_char;

    return 0;
}

// #define SGX_QUOTE_TYPE 0x0
// #define TDX_QUOTE_TYPE 0x81

//  00000000: fe 49 5c 2b 54 f8 c7 72 59 05 31 32 0b 33 a0 57
//  00000010: 6e b2 22 f0 db b3 6b ff 18 89 fb b0 20 97 ca 1e
//  00000020: e1 26 49 35 1e 11 a7 77 16 d8 aa 21 0c 4a 79 7a
//  00000030: fc 9b 6a d7 4e d6 d6 66 5f d2 16 80 69 e0 9e 4a

